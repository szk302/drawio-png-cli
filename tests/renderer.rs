#![cfg(unix)]
use assert_cmd::cargo::cargo_bin_cmd;
use drawio_png_cli::{png_data, render};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant},
};
use tempfile::{TempDir, tempdir};

const MODEL: &str = include_str!("fixtures/model.xml");
const MIXED: &str = include_str!("fixtures/mixed-pages.xml");

fn script(body: &str) -> (TempDir, PathBuf) {
    let directory = tempdir().unwrap();
    let path = directory.path().join("fake drawio");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        directory.path().join("plain.png"),
        include_bytes!("fixtures/plain.png"),
    )
    .unwrap();
    (directory, path)
}

#[test]
fn renderer_arguments_and_full_document_embedding() {
    let (directory, path) = script(
        r#"
[ "$1" = --export ] && [ "$2" = --format ] && [ "$3" = png ] && [ "$4" = --page-index ] && [ "$5" = 1 ] && [ "$6" = --output ] || exit 5
[ -f "$8" ] || exit 6
cp "$8" "$(dirname "$0")/received.xml"
cp "$(dirname "$0")/plain.png" "$7"
"#,
    );
    let output = directory.path().join("output.png");
    cargo_bin_cmd!("dip")
        .env("DIP_DRAWIO_PATH", &path)
        .args(["embed", "-o"])
        .arg(&output)
        .write_stdin(MIXED)
        .assert()
        .success()
        .stdout("");
    let xml = png_data::extract(&fs::read(output).unwrap()).unwrap();
    assert_eq!(xml.matches("<mxGraphModel").count(), 2);
    assert_eq!(
        fs::read_to_string(directory.path().join("received.xml")).unwrap(),
        xml
    );
}

#[test]
fn renderer_failures_and_invalid_output_are_errors() {
    for body in [
        "echo 'failure detail' >&2; exit 3",
        "exit 0",
        "echo broken > \"$7\"",
    ] {
        let (_directory, path) = script(body);
        assert!(render::render_with(&path, MODEL, Duration::from_secs(2)).is_err());
    }
}

#[test]
fn renderer_failure_does_not_overwrite_destination() {
    let (directory, path) = script("echo failure >&2; exit 1");
    let output = directory.path().join("output.png");
    fs::write(&output, b"original").unwrap();
    cargo_bin_cmd!("dip")
        .env("DIP_DRAWIO_PATH", &path)
        .args(["embed", "-o"])
        .arg(&output)
        .write_stdin(MODEL)
        .assert()
        .code(1);
    assert_eq!(fs::read(&output).unwrap(), b"original");
}

#[test]
fn timeout_terminates_renderer_promptly() {
    let (_directory, path) = script("exec sleep 30");
    let start = Instant::now();
    let error = render::render_with(&path, MODEL, Duration::from_millis(80)).unwrap_err();
    assert!(error.to_string().contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(3));
}

#[test]
fn path_discovery_works_and_bad_explicit_path_does_not_fall_back() {
    let (directory, path) = script("exit 7");
    fs::rename(&path, directory.path().join("drawio")).unwrap();
    let output = directory.path().join("output.png");
    let result = cargo_bin_cmd!("dip")
        .env_remove("DIP_DRAWIO_PATH")
        .env("PATH", directory.path())
        .args(["embed", "-o"])
        .arg(&output)
        .write_stdin(MODEL)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    assert!(String::from_utf8_lossy(&result).contains("Desktop failed"));
    let result = cargo_bin_cmd!("dip")
        .env("DIP_DRAWIO_PATH", "does-not-exist")
        .env("PATH", directory.path())
        .args(["embed", "-o"])
        .arg(&output)
        .write_stdin(MODEL)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    assert!(String::from_utf8_lossy(&result).contains("DIP_DRAWIO_PATH"));
}

#[test]
fn timeout_also_stops_renderer_helpers() {
    let (directory, path) = script(
        r#"
(sleep 0.4; echo escaped > "$(dirname "$0")/escaped") &
wait
"#,
    );
    assert!(render::render_with(&path, MODEL, Duration::from_millis(80)).is_err());
    std::thread::sleep(Duration::from_millis(500));
    assert!(!directory.path().join("escaped").exists());
}
