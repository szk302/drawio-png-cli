use anyhow::{Context, Result, bail, ensure};
use std::{
    env,
    ffi::OsStr,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{MAX_BYTES, png_data, storage};

pub const TIMEOUT: Duration = Duration::from_secs(60);

fn executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn discover() -> Result<PathBuf> {
    if let Some(path) = env::var_os("DIP_DRAWIO_PATH") {
        let path = PathBuf::from(path);
        ensure!(
            executable(&path),
            "DIP_DRAWIO_PATH is not an executable file: {}",
            path.display()
        );
        return path
            .canonicalize()
            .context("cannot resolve DIP_DRAWIO_PATH");
    }
    let names: &[&str] = if cfg!(windows) {
        &["drawio.exe", "draw.io.exe"]
    } else {
        &["drawio", "draw.io"]
    };
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            for name in names {
                let candidate = directory.join(name);
                if executable(&candidate) {
                    return Ok(candidate.canonicalize()?);
                }
            }
        }
    }
    let mut defaults = Vec::<PathBuf>::new();
    match env::consts::OS {
        "macos" => {
            defaults.push("/Applications/draw.io.app/Contents/MacOS/draw.io".into());
            if let Some(home) = env::var_os("HOME") {
                defaults.push(
                    PathBuf::from(home).join("Applications/draw.io.app/Contents/MacOS/draw.io"),
                );
            }
        }
        "windows" => {
            for key in ["ProgramFiles", "LOCALAPPDATA"] {
                if let Some(base) = env::var_os(key) {
                    let base = PathBuf::from(base);
                    defaults.push(base.join("draw.io/draw.io.exe"));
                    defaults.push(base.join("Programs/draw.io/draw.io.exe"));
                }
            }
        }
        _ => defaults.push("/opt/drawio/drawio".into()),
    }
    for path in defaults {
        if executable(&path) {
            return Ok(path);
        }
    }
    bail!(
        "draw.io Desktop not found; install it or set DIP_DRAWIO_PATH, or use --no-render (Chrome fallback is not yet supported)"
    )
}

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn log_excerpt(log: &mut File) -> String {
    let _ = log.seek(SeekFrom::Start(0));
    let mut bytes = Vec::new();
    let _ = log.take(8192).read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).trim().to_owned()
}

pub fn render(xml: &str) -> Result<Vec<u8>> {
    render_with(&discover()?, xml, TIMEOUT)
}

/// Kept independent of discovery so renderer failures can be tested without Desktop.
pub fn render_with(program: &Path, xml: &str, timeout: Duration) -> Result<Vec<u8>> {
    let directory = tempfile::Builder::new().prefix("dip-render-").tempdir()?;
    let input = directory.path().join("input.drawio");
    let output = directory.path().join("output.png");
    fs::write(&input, xml)?;
    let mut log = tempfile::tempfile()?;
    let child = Command::new(program)
        .args([
            OsStr::new("--export"),
            OsStr::new("--format"),
            OsStr::new("png"),
            OsStr::new("--page-index"),
            OsStr::new("0"),
            OsStr::new("--output"),
        ])
        .arg(&output)
        .arg(&input)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log.try_clone()?))
        .spawn()
        .with_context(|| format!("cannot start draw.io Desktop: {}", program.display()))?;
    let mut child = ChildGuard(child);
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.0.try_wait()? {
            ensure!(
                status.success(),
                "draw.io Desktop failed ({status}): {}",
                log_excerpt(&mut log)
            );
            break;
        }
        if Instant::now() >= deadline {
            bail!(
                "draw.io Desktop timed out after {} seconds: {}",
                timeout.as_secs_f64(),
                log_excerpt(&mut log)
            );
        }
        ensure!(
            log.metadata()?.len() <= MAX_BYTES as u64,
            "draw.io Desktop log exceeds 64 MiB limit"
        );
        thread::sleep(Duration::from_millis(20));
    }
    let png = storage::read(&output).with_context(|| {
        format!(
            "draw.io Desktop produced no readable PNG: {}",
            log_excerpt(&mut log)
        )
    })?;
    png_data::validate(&png).context("draw.io Desktop produced an invalid PNG")?;
    Ok(png)
}
