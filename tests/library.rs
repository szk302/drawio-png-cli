use assert_cmd::{Command, cargo::cargo_bin_cmd};
use base64::{Engine, engine::general_purpose::STANDARD};
use drawio_png_cli::{document, library};
use std::{fs, io::Write};
use tempfile::tempdir;

const MODEL: &str = include_str!("fixtures/model.xml");
const MIXED: &str = include_str!("fixtures/mixed-pages.xml");

fn dip() -> Command {
    cargo_bin_cmd!("dip")
}

/// Compresses a model the way draw.io stores library entries.
fn compress(xml: &str) -> String {
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(urlencoding::encode(xml).as_bytes())
        .unwrap();
    STANDARD.encode(encoder.finish().unwrap())
}

const ICON: &str = r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="2" style="shape=image;image=data:image/svg+xml,PHN2Zy8+" vertex="1" parent="1"><mxGeometry width="144" height="72" as="geometry"/></mxCell></root></mxGraphModel>"#;

// A group with a child, a labelled object and an edge between the two vertices.
const GROUP: &str = r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="g" value="" style="group" vertex="1" connectable="0" parent="1"><mxGeometry x="10" y="20" width="100" height="40" as="geometry"/></mxCell><mxCell id="c" value="child" style="rounded=1;" vertex="1" parent="g"><mxGeometry x="5" y="5" width="30" height="30" as="geometry"/></mxCell><object id="o" label="object" note="kept"><mxCell style="ellipse;" vertex="1" parent="1"><mxGeometry x="150" y="20" width="40" height="40" as="geometry"/></mxCell></object><mxCell id="e" style="edgeStyle=orthogonalEdgeStyle;" edge="1" parent="1" source="g" target="o"><mxGeometry relative="1" as="geometry"><mxPoint x="12" y="80" as="sourcePoint"/><Array as="points"><mxPoint x="120" y="90"/></Array><mxPoint x="5" y="6" as="offset"/></mxGeometry></mxCell></root></mxGraphModel>"#;

fn library_xml() -> String {
    let entries = serde_json::json!([
        {"xml": compress(ICON), "w": 144, "h": 72, "title": "Icon A"},
        {"xml": GROUP, "w": 200, "h": 80, "title": "Group\nB"},
        {"data": "data:image/png;base64,iVBORw0KGgo=", "w": 16, "h": 8, "title": "Raw", "aspect": "fixed"},
        {"data": "https://example.com/icon.svg", "w": 32, "h": 32, "title": "Linked"},
        {"xml": compress(ICON), "w": 10, "h": 10, "title": "Dup"},
        {"xml": compress(ICON), "w": 20, "h": 20, "title": "Dup"},
    ]);
    // draw.io escapes the JSON as element text.
    let text = entries
        .to_string()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!("<mxlibrary title=\"test\">{text}</mxlibrary>")
}

fn entries() -> Vec<library::Entry> {
    library::parse(library_xml().as_bytes()).unwrap()
}

fn placement(x: f64, y: f64) -> library::Placement<'static> {
    library::Placement {
        page: 1,
        x,
        y,
        width: None,
        height: None,
        id: None,
        label: None,
    }
}

fn cell<'a>(doc: &'a roxmltree::Document<'a>, id: &str) -> roxmltree::Node<'a, 'a> {
    doc.descendants()
        .find(|n| n.attribute("id") == Some(id))
        .unwrap_or_else(|| panic!("no cell {id}"))
}

fn geometry<'a>(node: roxmltree::Node<'a, 'a>) -> roxmltree::Node<'a, 'a> {
    let inner = if node.has_tag_name("mxCell") {
        node
    } else {
        node.children().find(|n| n.has_tag_name("mxCell")).unwrap()
    };
    inner
        .children()
        .find(|n| n.has_tag_name("mxGeometry"))
        .unwrap()
}

#[test]
fn list_prints_index_title_and_size_without_content() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("lib.xml");
    fs::write(&path, library_xml()).unwrap();
    let output = dip()
        .args(["library", "list"])
        .arg(&path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(
        text,
        "1\tIcon A\t144x72\n2\tGroup B\t200x80\n3\tRaw\t16x8\n4\tLinked\t32x32\n5\tDup\t10x10\n6\tDup\t20x20\n"
    );
    assert!(!text.contains("data:"));
    let filtered = dip()
        .args(["library", "list", "--filter", "ICON"])
        .arg(&path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(filtered).unwrap(), "1\tIcon A\t144x72\n");
}

#[test]
fn add_places_resizes_and_labels_a_single_cell() {
    let entries = entries();
    let entry = library::find(&entries, "icon a").unwrap();
    let mut options = placement(200.0, 30.0);
    options.width = Some(48.0);
    options.id = Some("icon");
    options.label = Some("Label & <b>");
    let (xml, ids) = library::add(MODEL, entry, &options).unwrap();
    assert_eq!(ids, ["icon"]);
    document::validate(&xml).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let icon = cell(&doc, "icon");
    assert_eq!(icon.attribute("value"), Some("Label & <b>"));
    assert_eq!(icon.attribute("parent"), Some("1"));
    assert!(
        icon.attribute("style")
            .unwrap()
            .contains("image=data:image/svg+xml,PHN2Zy8+")
    );
    let g = geometry(icon);
    // The aspect ratio is kept when only the width is given.
    let size = |n: &str| g.attribute(n).unwrap();
    assert_eq!(
        (size("x"), size("y"), size("width"), size("height")),
        ("200", "30", "48", "24")
    );
    // Existing cells are untouched.
    assert!(xml.starts_with(&MODEL[..MODEL.find("</root>").unwrap()]));
}

#[test]
fn add_remaps_ids_parents_and_edges_of_multi_cell_entries() {
    let entries = entries();
    let entry = library::find(&entries, "Group\nB").unwrap();
    let (xml, ids) = library::add(MODEL, entry, &placement(100.0, 200.0)).unwrap();
    document::validate(&xml).unwrap();
    // Top-level cells first: the group, the object and the edge.
    assert_eq!(ids, ["lib-1", "lib-3", "lib-4", "lib-2"]);
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let group = cell(&doc, "lib-1");
    let child = cell(&doc, "lib-2");
    let object = cell(&doc, "lib-3");
    let edge = cell(&doc, "lib-4");
    assert_eq!(child.attribute("parent"), Some("lib-1"));
    assert_eq!(group.attribute("parent"), Some("1"));
    assert_eq!(object.attribute("label"), Some("object"));
    assert_eq!(object.attribute("note"), Some("kept"));
    assert_eq!(
        geometry(object).parent().unwrap().attribute("parent"),
        Some("1")
    );
    assert_eq!(edge.attribute("source"), Some("lib-1"));
    assert_eq!(edge.attribute("target"), Some("lib-3"));
    // The entry's top-left corner (10, 20) moves to (100, 200); child
    // coordinates are relative to the group and stay.
    let xy = |n: roxmltree::Node<'_, '_>| {
        (
            n.attribute("x").unwrap().to_owned(),
            n.attribute("y").unwrap().to_owned(),
        )
    };
    assert_eq!(xy(geometry(group)), ("100".into(), "200".into()));
    assert_eq!(xy(geometry(object)), ("240".into(), "200".into()));
    assert_eq!(xy(geometry(child)), ("5".into(), "5".into()));
    let points: Vec<_> = geometry(edge)
        .descendants()
        .filter(|n| n.has_tag_name("mxPoint"))
        .map(xy)
        .collect();
    assert_eq!(
        points,
        [
            ("102".into(), "260".into()),
            ("210".into(), "270".into()),
            ("5".into(), "6".into()) // the label offset is relative
        ]
    );
    // A second add gets fresh IDs.
    let (again, ids) = library::add(&xml, entry, &placement(0.0, 0.0)).unwrap();
    document::validate(&again).unwrap();
    assert_eq!(ids, ["lib-5", "lib-7", "lib-8", "lib-6"]);
    // Labels and sizes need a single cell.
    let mut options = placement(0.0, 0.0);
    options.label = Some("x");
    assert!(library::add(MODEL, entry, &options).is_err());
    let mut options = placement(0.0, 0.0);
    options.width = Some(10.0);
    assert!(library::add(MODEL, entry, &options).is_err());
}

#[test]
fn add_turns_image_entries_into_image_cells() {
    let entries = entries();
    let entry = library::find(&entries, "Raw").unwrap();
    let (xml, ids) = library::add(MODEL, entry, &placement(1.0, 2.0)).unwrap();
    document::validate(&xml).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let image = cell(&doc, &ids[0]);
    // A style value cannot contain ';', so ";base64" is dropped like draw.io does.
    let style = image.attribute("style").unwrap();
    assert!(
        style.contains("image=data:image/png,iVBORw0KGgo="),
        "{style}"
    );
    assert!(style.contains("aspect=fixed"));
    let g = geometry(image);
    assert_eq!(
        (g.attribute("width"), g.attribute("height")),
        (Some("16"), Some("8"))
    );
    // Linked images are kept as URLs; rendering them needs --allow-network.
    let entry = library::find(&entries, "Linked").unwrap();
    let (xml, ids) = library::add(MODEL, entry, &placement(0.0, 0.0)).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let style = cell(&doc, &ids[0]).attribute("style").unwrap();
    assert!(
        style.contains("image=https://example.com/icon.svg"),
        "{style}"
    );
}

#[test]
fn add_targets_pages_and_expands_compressed_ones() {
    let entries = entries();
    let entry = library::find(&entries, "Icon A").unwrap();
    for page in [1, 2] {
        let mut options = placement(0.0, 0.0);
        options.page = page;
        let (xml, ids) = library::add(MIXED, entry, &options).unwrap();
        document::validate(&xml).unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let pages: Vec<_> = doc
            .descendants()
            .filter(|n| n.has_tag_name("diagram"))
            .collect();
        assert_eq!(pages.len(), 2);
        for (index, diagram) in pages.iter().enumerate() {
            let has = diagram
                .descendants()
                .any(|n| n.attribute("id") == Some(ids[0].as_str()));
            assert_eq!(has, index + 1 == page, "page {page}");
        }
    }
    let mut options = placement(0.0, 0.0);
    options.page = 3;
    assert!(library::add(MIXED, entry, &options).is_err());
}

#[test]
fn name_selection_requires_a_unique_match() {
    let entries = entries();
    assert!(library::find(&entries, "Dup").is_err());
    assert!(library::find(&entries, "missing").is_err());
    assert_eq!(library::find(&entries, "ICON A").unwrap().title, "Icon A");
}

#[test]
fn cli_add_prints_ids_and_keeps_output_on_errors() {
    let dir = tempdir().unwrap();
    let lib = dir.path().join("lib.xml");
    fs::write(&lib, library_xml()).unwrap();
    let output = dir.path().join("out.xml");
    let result = dip()
        .args(["library", "add"])
        .arg(&lib)
        .args(["--name", "Icon A", "--id", "icon", "--x", "5", "-o"])
        .arg(&output)
        .write_stdin(MODEL)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(result).unwrap(), "icon\n");
    let saved = fs::read(&output).unwrap();
    // Reusing the ID, an ambiguous name and a bad index all fail without writing.
    for args in [
        vec!["--name", "Icon A", "--id", "icon"],
        vec!["--name", "Dup"],
        vec!["--index", "9"],
        vec!["--index", "1", "--width", "0"],
    ] {
        dip()
            .args(["library", "add"])
            .arg(&lib)
            .args(&args)
            .arg("-i")
            .arg(&output)
            .arg("-o")
            .arg(&output)
            .assert()
            .code(1);
        assert_eq!(fs::read(&output).unwrap(), saved, "{args:?}");
    }
    dip()
        .args(["library", "add"])
        .arg(&lib)
        .args(["--name", "Icon A", "--index", "1", "-o"])
        .arg(&output)
        .assert()
        .code(2);
    // The entry content never reaches stdout.
    let list = dip()
        .args(["library", "add"])
        .arg(&lib)
        .args(["--index", "3", "-i"])
        .arg(&output)
        .arg("-o")
        .arg(&output)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(!String::from_utf8(list).unwrap().contains("data:"));
    document::validate(&fs::read_to_string(&output).unwrap()).unwrap();
}

#[test]
fn invalid_libraries_are_rejected() {
    for text in [
        "<mxlibrary>not json</mxlibrary>",
        "<other>[]</other>",
        r#"<mxlibrary>[{"title":"no content"}]</mxlibrary>"#,
    ] {
        assert!(library::parse(text.as_bytes()).is_err(), "{text}");
    }
    let broken = r#"<mxlibrary>[{"xml":"not base64!","title":"x"}]</mxlibrary>"#;
    let entries = library::parse(broken.as_bytes()).unwrap();
    assert!(library::add(MODEL, &entries[0], &placement(0.0, 0.0)).is_err());
}
