use anyhow::Result;
use clap::{Parser, Subcommand};
use drawio_png_cli::{chromium::ChromiumMode, document, fonts, library, png_data, render, storage};
use std::{
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(
    name = "dip",
    version,
    about = "Edit draw.io diagrams embedded in PNG images"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print licenses and notices for dip, dependencies, and bundled assets
    Licenses,
    /// Extract editable, uncompressed XML from a draw.io PNG
    Extract {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Embed XML into a PNG, rendering its first page unless --no-render is set
    #[command(
        after_help = "Renderer environment:\n  DIP_DRAWIO_PATH / DIP_DRAWIO_ARGS  Desktop executable and options\n  DIP_CHROME_PATH / CHROME_PATH     Chromium or Chrome executable\n  DIP_CHROMIUM_MODE                 raw, desktop, or vscode (default: vscode)\n  DIP_CHROME_ARGS                   Additional POSIX-quoted options (no shell expansion)\n  DIP_DRAWIO_WEB_PATH               Local draw.io webapp directory (default: bundled assets)"
    )]
    Embed {
        /// XML file (omit to read stdin)
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
        /// Preserve this PNG's pixels; requires --no-render
        #[arg(short, long, requires = "no_render")]
        base_image: Option<PathBuf>,
        /// Update metadata only; create a transparent 1x1 PNG if no base is given
        #[arg(long)]
        no_render: bool,
        /// Rendering backend (auto prefers Desktop, then Chromium)
        #[arg(long, value_enum, conflicts_with = "no_render")]
        renderer: Option<render::Renderer>,
        /// Chromium output mode (overrides DIP_CHROMIUM_MODE; default: vscode)
        #[arg(long, value_enum, conflicts_with = "no_render")]
        chromium_mode: Option<ChromiumMode>,
        /// Allow external HTTP(S) images and fonts in Chromium
        #[arg(long, conflicts_with = "no_render")]
        allow_network: bool,
        /// Set and save the font for cells without an explicit fontFamily
        #[arg(long, value_parser = fonts::parse_family, conflicts_with = "no_validate")]
        default_font: Option<String>,
        /// Append a fallback family (repeatable, in priority order); requires --default-font
        #[arg(long, value_parser = fonts::parse_family, requires = "default_font")]
        fallback_font: Vec<String>,
        /// Debug only: skip draw.io XML validation
        #[arg(long)]
        no_validate: bool,
    },
    /// Validate XML or a draw.io PNG (all pages)
    Validate { input: PathBuf },
    /// Insert a library entry into a page of an uncompressed draw.io XML file; prints the new cell IDs
    #[command(after_help = LIBRARY_ENV)]
    Insert {
        #[command(flatten)]
        source: LibrarySource,
        /// Entry title (exact, or a unique case-insensitive match)
        #[arg(long, required_unless_present = "index", conflicts_with = "index")]
        name: Option<String>,
        /// Entry index in its library as shown by `library search` (starting at 1)
        #[arg(long)]
        index: Option<usize>,
        /// XML file (omit to read stdin)
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
        /// Page number, starting at 1
        #[arg(long, default_value_t = 1)]
        page: usize,
        /// Left edge of the inserted shapes
        #[arg(long, default_value_t = 0.0)]
        x: f64,
        /// Top edge of the inserted shapes
        #[arg(long, default_value_t = 0.0)]
        y: f64,
        /// Width of a single-cell entry (keeps the aspect ratio if --height is omitted)
        #[arg(long)]
        width: Option<f64>,
        /// Height of a single-cell entry (keeps the aspect ratio if --width is omitted)
        #[arg(long)]
        height: Option<f64>,
        /// ID for the top-level cell; other cells get "<ID>-<n>"
        #[arg(long)]
        id: Option<String>,
        /// Label of the top-level cell
        #[arg(long)]
        label: Option<String>,
    },
    /// Inspect draw.io custom libraries (<mxlibrary> files) without changing anything
    Library {
        #[command(subcommand)]
        command: LibraryCommand,
    },
}

const LIBRARY_ENV: &str = "Library environment:\n  DIP_LIBRARY_PATH  Library files or directories of *.xml files, separated like PATH;\n                    used unless --library-file is given";

/// Which libraries to use.
#[derive(clap::Args)]
struct LibrarySource {
    /// Library name: its file name without .xml
    #[arg(long)]
    library: Option<String>,
    /// Library file to use instead of DIP_LIBRARY_PATH
    #[arg(long, value_name = "PATH", conflicts_with = "library")]
    library_file: Option<PathBuf>,
}

#[derive(Subcommand)]
enum LibraryCommand {
    /// List libraries as name, entry count and title
    #[command(visible_alias = "ls", after_help = LIBRARY_ENV)]
    List {
        /// Library file to use instead of DIP_LIBRARY_PATH
        #[arg(long, value_name = "PATH")]
        library_file: Option<PathBuf>,
    },
    /// Show details of one library
    #[command(after_help = LIBRARY_ENV)]
    Show {
        /// Library name: its file name without .xml
        name: String,
        /// Library file to use instead of DIP_LIBRARY_PATH
        #[arg(long, value_name = "PATH")]
        library_file: Option<PathBuf>,
    },
    /// Search entries; prints library, index, title and size, without their content
    #[command(after_help = LIBRARY_ENV)]
    Search {
        /// Text the title must contain (case-insensitive); omit to list every entry
        query: Option<String>,
        #[command(flatten)]
        source: LibrarySource,
    },
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Licenses => {
            io::stdout()
                .lock()
                .write_all(drawio_png_cli::BUNDLED_LICENSES.as_bytes())?;
        }
        Command::Extract { input, output } => {
            let data = storage::read(&input)?;
            let xml = document::normalize(&png_data::extract(&data)?)?;
            if let Some(path) = output {
                storage::atomic_write(&path, xml.as_bytes())?;
            } else {
                io::stdout().lock().write_all(xml.as_bytes())?;
            }
        }
        Command::Embed {
            input,
            output,
            base_image,
            no_render,
            no_validate,
            renderer,
            chromium_mode,
            allow_network,
            default_font,
            fallback_font,
        } => {
            let bytes = if let Some(path) = input {
                storage::read(&path)?
            } else {
                storage::read_limited(io::stdin().lock())?
            };
            let xml = document::utf8(&bytes)?;
            let xml = if no_validate {
                eprintln!("warning: draw.io XML validation is disabled");
                xml.to_owned()
            } else {
                document::validate(xml)?;
                document::normalize(xml)?
            };
            let xml = if let Some(default) = default_font {
                fonts::FontPolicy::new(&default, &fallback_font)?.apply(&xml)?
            } else {
                xml
            };
            let base = if no_render {
                eprintln!(
                    "warning: PNG pixels are not synchronized with the diagram (--no-render)"
                );
                if let Some(path) = base_image {
                    storage::read(&path)?
                } else {
                    png_data::transparent()?
                }
            } else {
                render::render_selected_with_mode(
                    &xml,
                    renderer.unwrap_or_default(),
                    allow_network,
                    chromium_mode,
                )?
            };
            let png = png_data::embed(&base, &xml)?;
            storage::atomic_write(&output, &png)?;
        }
        Command::Validate { input } => {
            let data = storage::read(&input)?;
            let xml = if data.starts_with(png_data::SIGNATURE) {
                png_data::extract(&data)?
            } else {
                document::utf8(&data)?.to_owned()
            };
            document::validate(&xml)?;
        }
        Command::Library { command } => library_command(command)?,
        Command::Insert {
            source,
            name,
            index,
            input,
            output,
            page,
            x,
            y,
            width,
            height,
            id,
            label,
        } => insert(
            source,
            name,
            index,
            input,
            output,
            library::Placement {
                page,
                x,
                y,
                width,
                height,
                id: id.as_deref(),
                label: label.as_deref(),
            },
        )?,
    }
    Ok(())
}

/// The libraries to use: the given file, or those on DIP_LIBRARY_PATH,
/// optionally only the one with the given name.
fn libraries(file: Option<PathBuf>, name: Option<&str>) -> Result<Vec<library::Library>> {
    let libraries = if let Some(file) = file {
        vec![library::open(&file)?]
    } else {
        let path = std::env::var_os("DIP_LIBRARY_PATH")
            .filter(|p| !p.is_empty())
            .ok_or_else(|| anyhow::anyhow!("set DIP_LIBRARY_PATH or pass --library-file"))?;
        let (libraries, warnings) = library::search(&path)?;
        for warning in warnings {
            eprintln!("warning: skipped {warning}");
        }
        libraries
    };
    let Some(name) = name else {
        anyhow::ensure!(
            !libraries.is_empty(),
            "no libraries found in DIP_LIBRARY_PATH"
        );
        return Ok(libraries);
    };
    let available: Vec<_> = libraries.iter().map(|l| l.name.clone()).collect();
    let selected: Vec<_> = libraries.into_iter().filter(|l| l.name == name).collect();
    anyhow::ensure!(
        !selected.is_empty(),
        "no library named {name:?}; available: {}",
        available.join(", ")
    );
    Ok(selected)
}

/// Keeps one record per line.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// Writes to stdout; a consumer such as `head` may stop reading early.
fn print(text: &str) -> Result<()> {
    match io::stdout().lock().write_all(text.as_bytes()) {
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => Ok(other?),
    }
}

fn library_command(command: LibraryCommand) -> Result<()> {
    let mut out = String::new();
    match command {
        LibraryCommand::List { library_file } => {
            for library in libraries(library_file, None)? {
                out.push_str(&format!(
                    "{}\t{}\t{}\n",
                    library.name,
                    library.entries.len(),
                    library.title.as_deref().map_or("-".into(), one_line)
                ));
            }
        }
        LibraryCommand::Show { name, library_file } => {
            for library in libraries(library_file, Some(&name))? {
                out.push_str(&format!(
                    "name: {}\ntitle: {}\nfile: {}\nentries: {}\n",
                    library.name,
                    library.title.as_deref().map_or("-".into(), one_line),
                    library.path.display(),
                    library.entries.len()
                ));
            }
        }
        LibraryCommand::Search { query, source } => {
            let query = query.map(|q| q.to_lowercase());
            for library in libraries(source.library_file, source.library.as_deref())? {
                for (index, entry) in library.entries.iter().enumerate() {
                    if query
                        .as_ref()
                        .is_some_and(|q| !entry.title.to_lowercase().contains(q))
                    {
                        continue;
                    }
                    out.push_str(&format!(
                        "{}\t{}\t{}\t{}x{}\n",
                        library.name,
                        index + 1,
                        one_line(&entry.title),
                        entry.width,
                        entry.height
                    ));
                }
            }
        }
    }
    print(&out)
}

fn insert(
    source: LibrarySource,
    name: Option<String>,
    index: Option<usize>,
    input: Option<PathBuf>,
    output: PathBuf,
    placement: library::Placement<'_>,
) -> Result<()> {
    let libraries = libraries(source.library_file, source.library.as_deref())?;
    let entry = match (name, index) {
        (Some(name), _) => library::find(&libraries, &name)?.1,
        (None, Some(index)) => {
            let [library] = libraries.as_slice() else {
                anyhow::bail!("--index needs one library; use --library");
            };
            index
                .checked_sub(1)
                .and_then(|i| library.entries.get(i))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "library {:?} has {} entries; --index starts at 1",
                        library.name,
                        library.entries.len()
                    )
                })?
        }
        (None, None) => unreachable!("clap requires --name or --index"),
    };
    for (flag, value) in [
        ("--x", Some(placement.x)),
        ("--y", Some(placement.y)),
        ("--width", placement.width),
        ("--height", placement.height),
    ] {
        anyhow::ensure!(
            value.is_none_or(f64::is_finite),
            "{flag} must be a finite number"
        );
    }
    for (flag, value) in [("--width", placement.width), ("--height", placement.height)] {
        anyhow::ensure!(value.is_none_or(|v| v > 0.0), "{flag} must be positive");
    }
    let bytes = if let Some(path) = input {
        storage::read(&path)?
    } else {
        storage::read_limited(io::stdin().lock())?
    };
    let (xml, ids) = library::insert(document::utf8(&bytes)?, entry, &placement)?;
    storage::atomic_write(&output, xml.as_bytes())?;
    print(&ids.iter().map(|id| format!("{id}\n")).collect::<String>())
}

fn main() -> std::process::ExitCode {
    match run(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
