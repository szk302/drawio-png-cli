//! draw.io's built-in shapes, offered as the drawio/* libraries.
use assert_cmd::{Command, cargo::cargo_bin_cmd};
use drawio_png_cli::{document, png_data, shapes};
use std::fs;
use tempfile::tempdir;

const MODEL: &str = include_str!("fixtures/model.xml");

fn dip() -> Command {
    let mut command = cargo_bin_cmd!("dip");
    for name in [
        "DIP_DRAWIO_PATH",
        "DIP_DRAWIO_ARGS",
        "DIP_CHROME_PATH",
        "CHROME_PATH",
        "DIP_CHROME_ARGS",
        "DIP_CHROMIUM_MODE",
        "DIP_DRAWIO_WEB_PATH",
        "DIP_LIBRARY_PATH",
    ] {
        command.env_remove(name);
    }
    command
}

fn stdout(command: &mut Command) -> String {
    String::from_utf8(command.assert().success().get_output().stdout.clone()).unwrap()
}

fn stderr(command: &mut Command, code: i32) -> String {
    String::from_utf8(command.assert().code(code).get_output().stderr.clone()).unwrap()
}

#[test]
fn catalog_holds_cells_and_styles_but_no_images() {
    let catalog = include_bytes!("../assets/shapes/catalog.json.gz");
    let mut text = String::new();
    std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(&catalog[..]), &mut text)
        .unwrap();
    // The catalog only names shapes and image paths; icon artwork is never redistributed.
    assert!(!text.contains("data:image"));
    assert!(text.contains("96a916a337d13fc8bf622c8a67d422bd284eabe5"));
    let libraries = shapes::libraries().unwrap();
    assert!(libraries.len() > 300);
    assert!(libraries.iter().all(shapes::is_builtin));
    for name in ["aws4-compute", "azure2-compute", "gcp2-zones", "kubernetes"] {
        let name = format!("drawio/{name}");
        assert!(libraries.iter().any(|l| l.name == name), "{name}");
    }
}

#[test]
fn built_in_libraries_are_listed_and_searched_without_styles() {
    let list = stdout(dip().args(["library", "list"]));
    assert!(
        list.contains("drawio/aws4-compute\t139\tAWS / Compute\n"),
        "{list}"
    );
    let show = stdout(dip().args(["library", "show", "aws4-compute"]));
    assert_eq!(
        show,
        "name: drawio/aws4-compute\ntitle: AWS / Compute\nfile: (built in: draw.io 26.0.2 sidebar)\nentries: 139\n"
    );

    // Every library is searched, legacy palettes included.
    let found = stdout(dip().args(["library", "search", "LAMBDA"]));
    assert!(
        found.contains("drawio/aws4-compute\t16\tLambda\t78x78\n"),
        "{found}"
    );
    assert!(found.contains("drawio/aws3-compute\t"), "{found}");
    assert!(!found.contains("style") && !found.contains("mxgraph"));
    // The drawio/ prefix may be left out.
    for library in ["drawio/aws4-compute", "aws4-compute"] {
        assert_eq!(
            stdout(dip().args(["library", "search", "lambda", "--library", library])),
            "drawio/aws4-compute\t16\tLambda\t78x78\ndrawio/aws4-compute\t88\tLambda Function\t48x48\n"
        );
    }
    let missing = stderr(dip().args(["library", "search", "--library", "nope"]), 1);
    assert!(
        missing.contains("no library named \"nope\"; available: drawio/* (see `dip library list`)"),
        "{missing}"
    );
    // --no-builtin leaves nothing without DIP_LIBRARY_PATH.
    let none = stderr(dip().args(["library", "search", "--no-builtin"]), 1);
    assert!(none.contains("set DIP_LIBRARY_PATH"), "{none}");
}

#[test]
fn style_prints_one_line_for_single_cells_only() {
    let style = stdout(dip().args(["library", "style", "drawio/aws4-compute", "16"]));
    assert!(style.ends_with("shape=mxgraph.aws4.resourceIcon;resIcon=mxgraph.aws4.lambda;\n"));
    assert_eq!(style.lines().count(), 1);
    assert_eq!(
        stdout(dip().args(["library", "style", "aws4-compute", "16"])),
        style
    );
    let cards = stderr(
        dip().args(["library", "style", "drawio/gcp2-product-cards", "1"]),
        1,
    );
    assert!(
        cards.contains("`dip insert --library drawio/gcp2-product-cards --index 1`"),
        "{cards}"
    );
    for index in ["0", "79"] {
        let range = stderr(dip().args(["library", "style", "kubernetes", index]), 1);
        assert!(
            range.contains("library \"drawio/kubernetes\" has 78 entries"),
            "{range}"
        );
    }

    // Image entries of custom libraries carry their data only through insert.
    let directory = tempdir().unwrap();
    let file = directory.path().join("icons.xml");
    fs::write(
        &file,
        r#"<mxlibrary>[{"data":"data:image/png;base64,iVBORw0KGgo=","w":16,"h":16,"title":"Dot"}]</mxlibrary>"#,
    )
    .unwrap();
    let image = stderr(
        dip()
            .env("DIP_LIBRARY_PATH", &file)
            .args(["library", "style", "icons", "1"]),
        1,
    );
    assert!(
        image.contains("`dip insert --library icons --index 1`"),
        "{image}"
    );
    assert!(!image.contains("data:"));
}

#[test]
fn insert_adds_built_in_shapes_by_title_or_index() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("in.xml");
    let output = directory.path().join("out.xml");
    fs::write(&input, MODEL).unwrap();
    let insert = |args: &[&str]| {
        let mut command = dip();
        command
            .arg("insert")
            .args(args)
            .arg("-i")
            .arg(&input)
            .arg("-o")
            .arg(&output);
        command
    };

    let ids = stdout(&mut insert(&[
        "--library",
        "aws4-compute",
        "--name",
        "lambda",
        "--id",
        "fn",
        "--x",
        "40",
        "--y",
        "60",
        "--label",
        "Handler",
    ]));
    assert_eq!(ids, "fn\n");
    let xml = fs::read_to_string(&output).unwrap();
    document::validate(&xml).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let cell = doc
        .descendants()
        .find(|n| n.attribute("id") == Some("fn"))
        .unwrap();
    assert!(
        cell.attribute("style")
            .unwrap()
            .contains("shape=mxgraph.aws4.resourceIcon;resIcon=mxgraph.aws4.lambda;")
    );
    assert_eq!(cell.attribute("value"), Some("Handler"));
    let geometry = cell
        .children()
        .find(|n| n.has_tag_name("mxGeometry"))
        .unwrap();
    assert_eq!(geometry.attribute("x"), Some("40"));
    assert_eq!(geometry.attribute("width"), Some("78"));

    // Multi-cell entries get fresh IDs, as other library entries do.
    let ids = stdout(&mut insert(&[
        "--library",
        "drawio/gcp2-product-cards",
        "--index",
        "1",
    ]));
    assert!(ids.lines().count() > 1, "{ids}");
    document::validate(&fs::read_to_string(&output).unwrap()).unwrap();

    let ambiguous = stderr(&mut insert(&["--name", "Lambda"]), 1);
    assert!(
        ambiguous.contains("\"Lambda\" is in libraries ")
            && ambiguous.contains("\"drawio/aws4-compute\""),
        "{ambiguous}"
    );
    assert!(ambiguous.ends_with("; use --library\n"), "{ambiguous}");
    let hidden = stderr(
        &mut insert(&[
            "--name",
            "Lambda",
            "--library",
            "aws4-compute",
            "--no-builtin",
        ]),
        1,
    );
    assert!(hidden.contains("set DIP_LIBRARY_PATH"), "{hidden}");
}

#[test]
fn a_custom_library_named_like_a_built_in_one_needs_the_prefix() {
    let directory = tempdir().unwrap();
    let file = directory.path().join("kubernetes.xml");
    fs::write(
        &file,
        r#"<mxlibrary>[{"data":"data:image/png;base64,iVBORw0KGgo=","w":16,"h":16,"title":"Pod"}]</mxlibrary>"#,
    )
    .unwrap();
    let search = |library: &str| {
        let mut command = dip();
        command.env("DIP_LIBRARY_PATH", &file).args([
            "library",
            "search",
            "pod",
            "--library",
            library,
        ]);
        command
    };
    let both = stderr(&mut search("kubernetes"), 1);
    assert!(
        both.contains("use --library drawio/kubernetes for the built-in one, or --no-builtin"),
        "{both}"
    );
    assert!(stdout(&mut search("drawio/kubernetes")).starts_with("drawio/kubernetes\t"));
    assert_eq!(
        stdout(search("kubernetes").arg("--no-builtin")),
        "kubernetes\t1\tPod\t16x16\n"
    );
}

#[test]
fn preview_lays_out_labelled_entries_and_limits_their_number() {
    let libraries = shapes::libraries().unwrap();
    let kubernetes = libraries
        .iter()
        .find(|l| l.name == "drawio/kubernetes")
        .unwrap();
    let chosen: Vec<_> = (0..12).map(|i| (kubernetes, i)).collect();
    let xml = shapes::preview(&chosen).unwrap();
    document::validate(&xml).unwrap();
    assert!(xml.contains("drawio/kubernetes #1&#xa;"));
    assert!(xml.contains("drawio/kubernetes #12&#xa;"));

    let directory = tempdir().unwrap();
    let output = directory.path().join("preview.png");
    let preview = |args: &[&str]| {
        let mut command = dip();
        command
            .args(["library", "preview"])
            .args(args)
            .arg("-o")
            .arg(&output);
        command
    };
    let many = stderr(&mut preview(&["--library", "aws4-compute"]), 1);
    assert!(many.contains("139 entries match"), "{many}");
    let none = stderr(&mut preview(&["no such shape"]), 1);
    assert!(none.contains("no entries match"), "{none}");
    stderr(&mut preview(&[]), 2);
    assert!(!output.exists());
}

#[test]
#[ignore = "requires Chromium and draw.io 26.0.2 web assets; set DIP_TEST_CHROME_PATH and DIP_TEST_DRAWIO_WEB_PATH"]
fn preview_renders_built_in_shapes_with_web_assets() {
    let chrome = std::env::var_os("DIP_TEST_CHROME_PATH").expect("set DIP_TEST_CHROME_PATH");
    let web = std::env::var_os("DIP_TEST_DRAWIO_WEB_PATH").expect("set DIP_TEST_DRAWIO_WEB_PATH");
    let directory = tempdir().unwrap();
    let output = directory.path().join("preview.png");
    dip()
        .env("DIP_CHROME_PATH", chrome)
        .env(
            "DIP_CHROME_ARGS",
            std::env::var("DIP_TEST_CHROME_ARGS").unwrap_or_default(),
        )
        .env("DIP_DRAWIO_WEB_PATH", web)
        .args(["library", "preview", "lambda", "-o"])
        .arg(&output)
        .assert()
        .success();
    let png = fs::read(&output).unwrap();
    assert!(png.starts_with(png_data::SIGNATURE));
}
