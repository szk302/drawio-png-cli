use anyhow::Result;
use clap::{Parser, Subcommand};
use drawio_png_cli::{document, fonts, png_data, render, storage};
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
    /// Print bundled renderer notices and full license texts
    Licenses,
    /// Extract editable, uncompressed XML from a draw.io PNG
    Extract {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Embed XML into a PNG, rendering its first page unless --no-render is set
    #[command(
        after_help = "Renderer environment:\n  DIP_DRAWIO_PATH / DIP_DRAWIO_ARGS  Desktop executable and options\n  DIP_CHROME_PATH / CHROME_PATH     Chromium or Chrome executable\n  DIP_CHROME_ARGS                   Additional POSIX-quoted options (no shell expansion)\n  DIP_DRAWIO_WEB_PATH               Local draw.io webapp directory (default: bundled assets)"
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
                render::render_selected(&xml, renderer.unwrap_or_default(), allow_network)?
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
    }
    Ok(())
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
