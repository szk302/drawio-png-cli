use assert_cmd::{Command, cargo::cargo_bin_cmd};
use drawio_png_cli::{png_data, storage};
use std::fs;
use tempfile::tempdir;

const MODEL: &str = include_str!("fixtures/model.xml");
const MIXED: &str = include_str!("fixtures/mixed-pages.xml");
const PLAIN: &[u8] = include_bytes!("fixtures/plain.png");

fn dip() -> Command {
    let mut command = cargo_bin_cmd!("dip");
    command.env_remove("DIP_DRAWIO_PATH");
    command
}

#[test]
fn stdin_round_trip_and_validation() {
    let directory = tempdir().unwrap();
    let png = directory.path().join("日本語 space.drawio.png");
    dip()
        .args(["embed", "--no-render", "-o"])
        .arg(&png)
        .write_stdin(MIXED)
        .assert()
        .success();
    dip()
        .arg("validate")
        .arg(&png)
        .assert()
        .success()
        .stdout("")
        .stderr("");
    let result = dip()
        .arg("extract")
        .arg(&png)
        .assert()
        .success()
        .stderr("")
        .get_output()
        .stdout
        .clone();
    let xml = String::from_utf8(result).unwrap();
    assert_eq!(xml.matches("<mxGraphModel").count(), 2);
    assert!(xml.contains("日本語"));
    let output = directory.path().join("output.xml");
    dip()
        .arg("extract")
        .arg(&png)
        .arg("-o")
        .arg(&output)
        .assert()
        .success()
        .stdout("");
    assert_eq!(fs::read_to_string(output).unwrap(), xml);
}

#[test]
fn file_input_bom_and_same_path_base_update() {
    let directory = tempdir().unwrap();
    let xml = directory.path().join("input.xml");
    fs::write(&xml, format!("\u{feff}{MODEL}")).unwrap();
    dip().arg("validate").arg(&xml).assert().success();
    let png = directory.path().join("output.png");
    fs::write(&png, PLAIN).unwrap();
    dip()
        .args(["embed", "--no-render", "-i"])
        .arg(&xml)
        .arg("-b")
        .arg(&png)
        .arg("-o")
        .arg(&png)
        .assert()
        .success();
    assert_eq!(png_data::extract(&fs::read(png).unwrap()).unwrap(), MODEL);
}

#[test]
fn invalid_xml_never_changes_existing_output() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("output.png");
    fs::write(&path, PLAIN).unwrap();
    for invalid in [
        "<broken",
        "<mxfile/>",
        "<mxGraphModel><root/></mxGraphModel>",
    ] {
        dip()
            .args(["embed", "--no-render", "-o"])
            .arg(&path)
            .write_stdin(invalid)
            .assert()
            .code(1)
            .stdout("");
        assert_eq!(fs::read(&path).unwrap(), PLAIN);
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn invalid_renderer_is_an_error_without_touching_output() {
    let directory = tempdir().unwrap();
    let output = directory.path().join("output.png");
    fs::write(&output, PLAIN).unwrap();
    dip()
        .env("DIP_DRAWIO_PATH", directory.path().join("missing-renderer"))
        .args(["embed", "-o"])
        .arg(&output)
        .write_stdin(MODEL)
        .assert()
        .code(1);
    assert_eq!(fs::read(output).unwrap(), PLAIN);
}

#[test]
fn no_render_does_not_discover_a_renderer_or_reuse_output_pixels() {
    let directory = tempdir().unwrap();
    let output = directory.path().join("output.png");
    fs::write(&output, "not a PNG").unwrap();
    dip()
        .env("DIP_DRAWIO_PATH", "missing-renderer")
        .args(["embed", "--no-render", "-o"])
        .arg(&output)
        .write_stdin(MODEL)
        .assert()
        .success();
    png_data::validate(&fs::read(output).unwrap()).unwrap();
}

#[test]
fn debug_bypass_still_checks_png_and_saves_atomically() {
    let directory = tempdir().unwrap();
    let output = directory.path().join("output.png");
    dip()
        .args(["embed", "--no-render", "--no-validate", "-o"])
        .arg(&output)
        .write_stdin("<broken")
        .assert()
        .success();
    assert_eq!(
        png_data::extract(&fs::read(&output).unwrap()).unwrap(),
        "<broken"
    );
    dip().arg("validate").arg(&output).assert().code(1);
    let before = fs::read(&output).unwrap();
    let bad_base = directory.path().join("bad.png");
    fs::write(&bad_base, "bad").unwrap();
    dip()
        .args(["embed", "--no-render", "--no-validate", "-b"])
        .arg(&bad_base)
        .arg("-o")
        .arg(&output)
        .write_stdin(MODEL)
        .assert()
        .code(1);
    assert_eq!(fs::read(output).unwrap(), before);
}

#[test]
fn argument_errors_and_io_errors_have_distinct_exit_codes() {
    dip().arg("embed").assert().code(2);
    dip()
        .args(["embed", "-b", "base.png", "-o", "out.png"])
        .assert()
        .code(2);
    dip()
        .args(["validate", "does-not-exist.xml"])
        .assert()
        .code(1);
    dip().arg("--help").assert().success();
}

#[test]
fn atomic_save_failure_cleans_up_temporary_file() {
    let directory = tempdir().unwrap();
    let destination = directory.path().join("directory");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("keep"), "unchanged").unwrap();
    assert!(storage::atomic_write(&destination, b"replacement").is_err());
    assert_eq!(
        fs::read_to_string(destination.join("keep")).unwrap(),
        "unchanged"
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn atomic_save_keeps_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempdir().unwrap();
    let path = directory.path().join("private.xml");
    fs::write(&path, "old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    storage::atomic_write(&path, b"new").unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}
