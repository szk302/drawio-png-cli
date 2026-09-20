use assert_cmd::cargo::cargo_bin_cmd;
use drawio_png_cli::{chromium, document, png_data};
use std::{fs, path::PathBuf, time::Duration};

const MODEL: &str = include_str!("fixtures/model.xml");
const MIXED: &str = include_str!("fixtures/mixed-pages.xml");

fn dip() -> assert_cmd::Command {
    let mut command = cargo_bin_cmd!("dip");
    for name in [
        "DIP_DRAWIO_PATH",
        "DIP_DRAWIO_ARGS",
        "DIP_CHROME_PATH",
        "CHROME_PATH",
        "DIP_CHROME_ARGS",
        "DIP_DRAWIO_WEB_PATH",
    ] {
        command.env_remove(name);
    }
    command
}

#[test]
fn rendering_flags_conflict_with_no_render() {
    for flags in [
        vec!["--renderer", "chromium"],
        vec!["--renderer", "auto"],
        vec!["--allow-network"],
    ] {
        dip()
            .args(["embed", "--no-render", "-o", "unused.png"])
            .args(flags)
            .assert()
            .code(2);
    }
    dip()
        .args(["embed", "--renderer", "unknown", "-o", "unused.png"])
        .assert()
        .code(2);
}

#[test]
fn no_render_and_xml_commands_ignore_browser_environment() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output.png");
    let mut command = dip();
    command
        .env("DIP_CHROME_ARGS", "'unfinished")
        .env("DIP_CHROME_PATH", "missing")
        .env("DIP_DRAWIO_WEB_PATH", "missing")
        .args(["embed", "--no-render", "-o"])
        .arg(&output)
        .write_stdin(MODEL)
        .assert()
        .success();
    for command in ["extract", "validate"] {
        dip()
            .env("DIP_CHROME_ARGS", "'unfinished")
            .env("DIP_DRAWIO_WEB_PATH", "missing")
            .arg(command)
            .arg(&output)
            .assert()
            .success();
    }
}

#[test]
fn invalid_browser_configuration_preserves_output() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output.png");
    fs::write(&output, b"original").unwrap();
    for (name, value) in [
        ("DIP_CHROME_PATH", "missing"),
        ("CHROME_PATH", "missing"),
        ("DIP_CHROME_ARGS", "'unfinished"),
        ("DIP_CHROME_ARGS", "--user-data-dir=/tmp/profile"),
    ] {
        dip()
            .env(name, value)
            .args(["embed", "--renderer", "chromium", "-o"])
            .arg(&output)
            .write_stdin(MODEL)
            .assert()
            .code(1);
        assert_eq!(fs::read(&output).unwrap(), b"original");
    }
}

#[test]
fn bundled_license_texts_are_available_without_a_renderer() {
    let result = dip()
        .arg("licenses")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(result).unwrap();
    for notice in [
        "Apache License",
        "Felix Gnass",
        "Preet Shihn",
        "Mark Adler",
        "Cure53",
    ] {
        assert!(text.contains(notice), "missing {notice}");
    }
}

fn pixels(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buffer).unwrap();
    buffer.truncate(info.buffer_size());
    (info.width, info.height, buffer)
}
fn browser() -> (PathBuf, Vec<String>) {
    let program = std::env::var_os("DIP_TEST_CHROME_PATH").expect("set DIP_TEST_CHROME_PATH");
    let args =
        shell_words::split(&std::env::var("DIP_TEST_CHROME_ARGS").unwrap_or_default()).unwrap();
    (PathBuf::from(program), args)
}
fn render(xml: &str, root: Option<PathBuf>, network: bool) -> anyhow::Result<Vec<u8>> {
    let (program, args) = browser();
    chromium::render_with(&program, xml, Duration::from_secs(20), &args, root, network)
}

#[test]
#[ignore = "requires Chromium; set DIP_TEST_CHROME_PATH"]
fn real_chromium_renders_first_page_and_preserves_xml() {
    let (program, args) = browser();
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("日本語 diagram.png");
    dip()
        .env("DIP_CHROME_PATH", program)
        .env("DIP_CHROME_ARGS", shell_words::join(args))
        .env("DIP_DRAWIO_PATH", "invalid-but-ignored")
        .args(["embed", "--renderer", "chromium", "-o"])
        .arg(&output)
        .write_stdin(MIXED.replace("width=\"100\"", "width=\"300\""))
        .assert()
        .success();
    let png = fs::read(output).unwrap();
    let xml = png_data::extract(&png).unwrap();
    document::validate(&xml).unwrap();
    assert_eq!(xml.matches("<mxGraphModel").count(), 2);
    assert!(xml.contains("width=\"300\""));
    let actual = pixels(&png);
    assert!(actual.0 > 1 && actual.1 > 1);
    assert_eq!(actual, pixels(&render(MODEL, None, false).unwrap()));

    // Desktop 31.4.5 exports this font-independent rectangle at 104x44.
    let geometry = r##"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="2" vertex="1" parent="1" style="rounded=0;fillColor=#dae8fc;strokeColor=#6c8ebf;"><mxGeometry x="10" y="20" width="100" height="40" as="geometry"/></mxCell></root></mxGraphModel>"##;
    let actual = pixels(&render(geometry, None, false).unwrap());
    assert_eq!((actual.0, actual.1), (104, 44));
    assert_eq!(
        actual,
        pixels(include_bytes!("fixtures/geometry-desktop.png"))
    );
}

#[test]
#[ignore = "requires Chromium; set DIP_TEST_CHROME_PATH"]
fn real_chromium_local_assets_and_unsupported_content() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for (name, data) in [
        (
            "js/viewer.min.js",
            &include_bytes!("../assets/drawio/js_viewer.min.js.gz")[..],
        ),
        (
            "js/export-init.js",
            &include_bytes!("../assets/drawio/js_export-init.js.gz")[..],
        ),
        (
            "js/export.js",
            &include_bytes!("../assets/drawio/js_export.js.gz")[..],
        ),
        (
            "mxgraph/css/common.css",
            &include_bytes!("../assets/drawio/mxgraph_css_common.css.gz")[..],
        ),
    ] {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            drawio_png_cli::storage::read_limited(flate2::read::GzDecoder::new(data)).unwrap(),
        )
        .unwrap();
    }
    fs::write(
        root.join("export3.html"),
        include_str!("../assets/chromium.html"),
    )
    .unwrap();
    assert_eq!(
        pixels(&render(MODEL, Some(root.to_owned()), false).unwrap()),
        pixels(&render(MODEL, None, false).unwrap())
    );
    fs::remove_file(root.join("js/viewer.min.js")).unwrap();
    assert!(
        render(MODEL, Some(root.to_owned()), false)
            .unwrap_err()
            .to_string()
            .contains("Chromium")
    );
    let unknown = MODEL.replace(
        "vertex=\"1\"",
        "vertex=\"1\" style=\"shape=mxgraph.missing.shape;\"",
    );
    assert!(
        format!("{:#}", render(&unknown, None, false).unwrap_err()).contains("Unsupported shape")
    );
    let math = MODEL.replace("<mxGraphModel ", "<mxGraphModel math=\"1\" ");
    assert!(format!("{:#}", render(&math, None, false).unwrap_err()).contains("Math"));
    let huge = MODEL.replace("width=\"100\"", "width=\"20000000\"");
    assert!(render(&huge, None, false).is_err());
    // The final 3004x1504 PNG fits in 64 MiB, but its 2x capture does not.
    let capture_too_large = MODEL
        .replace("width=\"100\"", "width=\"3000\"")
        .replace("height=\"40\"", "height=\"1500\"");
    assert!(
        format!("{:#}", render(&capture_too_large, None, false).unwrap_err())
            .contains("2x Chromium capture")
    );
    let broken_image = MODEL.replace(
        "vertex=\"1\"",
        "vertex=\"1\" style=\"shape=image;image=data:image/png,aW52YWxpZA==;\"",
    );
    assert!(render(&broken_image, None, false).is_err());
}

#[test]
#[ignore = "requires Chromium; set DIP_TEST_CHROME_PATH"]
fn real_chromium_deadline_covers_javascript_execution() {
    let (program, args) = browser();
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("export3.html"),
        "<!doctype html><script>while (true) {}</script>",
    )
    .unwrap();
    let start = std::time::Instant::now();
    let error = chromium::render_with(
        &program,
        MODEL,
        Duration::from_secs(2),
        &args,
        Some(dir.path().to_owned()),
        false,
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
#[ignore = "requires Chromium; set DIP_TEST_CHROME_PATH"]
fn real_chromium_html_embedded_images_and_network_policy() {
    use base64::Engine;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255, 0, 0, 255])
            .unwrap();
    }
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/red.png", server.server_addr());
    let requests = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let counter = requests.clone();
    let signal = stop.clone();
    let data = png.clone();
    let worker = std::thread::spawn(move || {
        while !signal.load(Ordering::Relaxed) {
            if let Ok(Some(request)) = server.recv_timeout(Duration::from_millis(50)) {
                counter.fetch_add(1, Ordering::Relaxed);
                let _ = request.respond(tiny_http::Response::from_data(data.clone()).with_header(
                    tiny_http::Header::from_bytes("Content-Type", "image/png").unwrap(),
                ));
            }
        }
    });
    let image_model = |url: &str| {
        MODEL.replace(
            "vertex=\"1\"",
            &format!("vertex=\"1\" style=\"shape=image;image={url};html=1;\""),
        )
    };
    let external = image_model(&url);
    let blocked = render(&external, None, false);
    let blocked_count = requests.load(Ordering::Relaxed);
    let allowed = render(&external, None, true);
    let allowed_count = requests.load(Ordering::Relaxed);
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();
    assert!(blocked.is_err());
    assert_eq!(blocked_count, 0);
    assert!(allowed_count > 0);
    let embedded = image_model(&format!(
        "data:image/png,{}",
        base64::engine::general_purpose::STANDARD.encode(&png)
    ));
    let actual = render(&embedded, None, false).unwrap();
    assert_eq!(pixels(&actual), pixels(&allowed.unwrap()));
    assert!(
        pixels(&actual)
            .2
            .as_chunks::<4>()
            .0
            .contains(&[255, 0, 0, 255])
    );
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, time::Instant};
    fn script(root: &std::path::Path, name: &str, body: &str) -> PathBuf {
        let path = root.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
    #[test]
    fn explicit_browser_path_takes_precedence_and_desktop_does_not_fall_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = script(dir.path(), "chrome", "echo browser-selected >&2; exit 4");
        let output = dir.path().join("out.png");
        fs::write(&output, b"original").unwrap();
        let result = dip()
            .env("DIP_CHROME_PATH", &path)
            .env("CHROME_PATH", "missing")
            .args(["embed", "--renderer", "chromium", "-o"])
            .arg(&output)
            .write_stdin(MODEL)
            .assert()
            .code(1)
            .get_output()
            .stderr
            .clone();
        assert!(String::from_utf8_lossy(&result).contains("browser-selected"));
        let desktop = script(dir.path(), "desktop", "echo desktop-selected >&2; exit 4");
        let result = dip()
            .env("DIP_CHROME_PATH", &path)
            .env("DIP_DRAWIO_PATH", desktop)
            .env("DIP_CHROME_ARGS", "'unfinished")
            .args(["embed", "-o"])
            .arg(&output)
            .write_stdin(MODEL)
            .assert()
            .code(1)
            .get_output()
            .stderr
            .clone();
        assert!(String::from_utf8_lossy(&result).contains("desktop-selected"));
        assert!(!String::from_utf8_lossy(&result).contains("browser-selected"));
        assert_eq!(fs::read(&output).unwrap(), b"original");
    }

    #[test]
    fn browser_options_are_literal_and_invalid_options_never_launch() {
        let dir = tempfile::tempdir().unwrap();
        let path = script(
            dir.path(),
            "chromium",
            r#"
[ "$1" = '--label=日本語 space' ] || exit 10
[ "$2" = '--literal=$(touch injected)' ] || exit 11
touch "$(dirname "$0")/started"
echo literal-arguments-ok >&2
exit 7
"#,
        );
        let output = dir.path().join("out.png");
        let result = dip()
            .env("DIP_CHROME_PATH", &path)
            .env(
                "DIP_CHROME_ARGS",
                "--label='日本語 space' '--literal=$(touch injected)'",
            )
            .current_dir(dir.path())
            .args(["embed", "--renderer", "chromium", "-o"])
            .arg(&output)
            .write_stdin(MODEL)
            .assert()
            .code(1)
            .get_output()
            .stderr
            .clone();
        assert!(String::from_utf8_lossy(&result).contains("literal-arguments-ok"));
        assert!(!dir.path().join("injected").exists());
        fs::remove_file(dir.path().join("started")).unwrap();
        dip()
            .env("DIP_CHROME_PATH", &path)
            .env("DIP_CHROME_ARGS", "'unfinished")
            .args(["embed", "--renderer", "chromium", "-o"])
            .arg(&output)
            .write_stdin(MODEL)
            .assert()
            .code(1);
        assert!(!dir.path().join("started").exists());
    }
    #[test]
    fn browser_timeout_stops_helpers_and_removes_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = script(
            dir.path(),
            "chromium",
            r#"
for arg in "$@"; do case "$arg" in --user-data-dir=*) echo "${arg#--user-data-dir=}" > "$(dirname "$0")/profile";; esac; done
(sleep 0.5; touch "$(dirname "$0")/escaped") &
wait
"#,
        );
        let start = Instant::now();
        let error =
            chromium::render_with(&path, MODEL, Duration::from_millis(150), &[], None, false)
                .unwrap_err();
        assert!(format!("{error:#}").contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(600));
        assert!(!dir.path().join("escaped").exists());
        let profile = fs::read_to_string(dir.path().join("profile")).unwrap();
        assert!(!PathBuf::from(profile.trim()).exists());
    }
}
