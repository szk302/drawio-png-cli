//! Headless Chromium controlled directly through the Chrome DevTools Protocol.
use crate::{
    MAX_BYTES,
    chromium_assets::AssetServer,
    render::{ChildGuard, TIMEOUT, executable, log_excerpt},
    resample,
};
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    env, fs,
    io::ErrorKind,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket, protocol::WebSocketConfig};

pub fn discover() -> Result<PathBuf> {
    for key in ["DIP_CHROME_PATH", "CHROME_PATH"] {
        if let Some(path) = env::var_os(key) {
            let path = PathBuf::from(path);
            ensure!(
                executable(&path),
                "{key} is not an executable file: {}",
                path.display()
            );
            return path
                .canonicalize()
                .with_context(|| format!("cannot resolve {key}"));
        }
    }
    let names: &[&str] = if cfg!(windows) {
        &["chromium.exe", "chrome.exe"]
    } else {
        &[
            "chromium",
            "chromium-browser",
            "google-chrome",
            "google-chrome-stable",
            "chrome",
        ]
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
    let mut paths: Vec<PathBuf> = Vec::new();
    match env::consts::OS {
        "macos" => {
            for base in [
                Some(PathBuf::from("/Applications")),
                env::var_os("HOME").map(|h| PathBuf::from(h).join("Applications")),
            ]
            .into_iter()
            .flatten()
            {
                paths.push(base.join("Google Chrome.app/Contents/MacOS/Google Chrome"));
                paths.push(base.join("Chromium.app/Contents/MacOS/Chromium"));
            }
        }
        "windows" => {
            for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
                if let Some(base) = env::var_os(key) {
                    paths.push(PathBuf::from(&base).join("Google/Chrome/Application/chrome.exe"));
                    paths.push(PathBuf::from(base).join("Chromium/Application/chrome.exe"));
                }
            }
        }
        _ => paths.extend([
            PathBuf::from("/usr/bin/chromium"),
            PathBuf::from("/usr/bin/google-chrome"),
        ]),
    }
    paths
        .into_iter()
        .find(|p| executable(p))
        .context("Chromium/Chrome not found; set DIP_CHROME_PATH or use --no-render")
}

fn extra_args() -> Result<Vec<String>> {
    let Some(value) = env::var_os("DIP_CHROME_ARGS") else {
        return Ok(Vec::new());
    };
    let value = value
        .to_str()
        .context("DIP_CHROME_ARGS must be valid Unicode")?;
    let args =
        shell_words::split(value).context("invalid DIP_CHROME_ARGS: check quotes and escapes")?;
    for arg in &args {
        // These options bypass process ownership, the local transport or network policy.
        let key = arg.split('=').next().unwrap_or(arg);
        ensure!(
            arg.starts_with("--")
                && arg != "--"
                && !matches!(
                    key,
                    "--remote-debugging-port"
                        | "--remote-debugging-pipe"
                        | "--remote-debugging-address"
                        | "--user-data-dir"
                        | "--headless"
                        | "--disable-web-security"
                        | "--disable-site-isolation-trials"
                        | "--disable-features"
                        | "--load-extension"
                        | "--disable-extensions-except"
                        | "--proxy-server"
                        | "--proxy-pac-url"
                        | "--host-resolver-rules"
                        | "--no-startup-window"
                ),
            "unsupported DIP_CHROME_ARGS option: {arg}; dip manages the profile, transport and network policy"
        );
    }
    Ok(args)
}

pub fn render(xml: &str, allow_network: bool) -> Result<Vec<u8>> {
    let args = extra_args()?;
    let program = discover()?;
    render_with(
        &program,
        xml,
        TIMEOUT,
        &args,
        env::var_os("DIP_DRAWIO_WEB_PATH").map(PathBuf::from),
        allow_network,
    )
}

/// Explicit inputs allow isolated browser integration tests without global environment edits.
pub fn render_with(
    program: &Path,
    xml: &str,
    timeout: Duration,
    args: &[String],
    web_root: Option<PathBuf>,
    allow_network: bool,
) -> Result<Vec<u8>> {
    ensure!(xml.len() <= MAX_BYTES, "XML exceeds 64 MiB limit");
    let deadline = Instant::now() + timeout;
    let profile = tempfile::Builder::new().prefix("dip-chromium-").tempdir()?;
    let server = AssetServer::start(web_root, allow_network)?;
    let mut log = tempfile::tempfile()?;
    let mut command = Command::new(program);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let child = command
        .args(args)
        .args([
            "--headless",
            "--remote-debugging-address=127.0.0.1",
            "--remote-debugging-port=0",
            "--disable-background-networking",
            "--disable-component-update",
            "--disable-sync",
            "--disable-extensions",
            "--no-first-run",
            "--no-default-browser-check",
        ])
        .arg(format!("--user-data-dir={}", profile.path().display()))
        .arg("about:blank")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log.try_clone()?))
        .spawn()
        .with_context(|| format!("cannot start Chromium: {}", program.display()))?;
    let mut child = ChildGuard {
        child,
        finished: false,
    };
    #[cfg(windows)]
    let _job = WindowsJob::assign(&child.child)?;
    let result = (|| -> Result<Vec<u8>> {
        let (port, endpoint) = loop {
            remaining(deadline)?;
            if let Some(status) = child.child.try_wait()? {
                bail!("Chromium exited before startup ({status})");
            }
            ensure!(
                log.metadata()?.len() <= MAX_BYTES as u64,
                "Chromium log exceeds 64 MiB limit"
            );
            if let Ok(value) = fs::read_to_string(profile.path().join("DevToolsActivePort")) {
                let mut lines = value.lines();
                if let (Some(port), Some(endpoint)) = (lines.next(), lines.next()) {
                    let port: u16 = port.parse().context("invalid Chromium debugging port")?;
                    ensure!(
                        endpoint.starts_with("/devtools/browser/"),
                        "invalid Chromium debugging endpoint"
                    );
                    break (port, endpoint.to_owned());
                }
            }
            thread::sleep(Duration::from_millis(20));
        };
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let stream = TcpStream::connect_timeout(&address, remaining(deadline)?)?;
        stream.set_read_timeout(Some(remaining(deadline)?))?;
        stream.set_write_timeout(Some(remaining(deadline)?))?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_BYTES * 2))
            .max_frame_size(Some(MAX_BYTES * 2));
        let (socket, _) = tungstenite::client::client_with_config(
            format!("ws://127.0.0.1:{port}{endpoint}"),
            stream,
            Some(config),
        )
        .map_err(|e| anyhow::anyhow!("Chromium CDP handshake failed: {e}"))?;
        let mut cdp = Cdp {
            socket,
            next_id: 0,
            session: None,
            deadline,
            origin: server.origin.clone(),
            allow_network,
            transferred: 0,
            log: log.try_clone()?,
        };
        let target = cdp.call("Target.createTarget", json!({"url":"about:blank"}))?;
        let attached = cdp.call(
            "Target.attachToTarget",
            json!({"targetId": target["targetId"], "flatten":true}),
        )?;
        cdp.session = Some(
            attached["sessionId"]
                .as_str()
                .context("missing CDP session")?
                .into(),
        );
        cdp.call("Page.enable", json!({}))?;
        cdp.call("Network.enable", json!({}))?;
        cdp.call("Network.setBypassServiceWorker", json!({"bypass":true}))?;
        cdp.call("Fetch.enable", json!({"patterns":[{"urlPattern":"*"}]}))?;
        cdp.call(
            "Page.addScriptToEvaluateOnNewDocument",
            json!({"source":include_str!("../assets/chromium-init.js")}),
        )?;
        cdp.call(
            "Emulation.setDefaultBackgroundColorOverride",
            json!({"color":{"r":0,"g":0,"b":0,"a":0}}),
        )?;
        cdp.call(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":800,"height":600,"deviceScaleFactor":resample::SCALE,"mobile":false}),
        )?;
        let navigation = cdp.call(
            "Page.navigate",
            json!({"url": format!("{}/export3.html", server.origin)}),
        )?;
        ensure!(
            navigation.get("errorText").is_none(),
            "cannot load draw.io assets: {navigation}"
        );
        loop {
            if cdp
                .eval("document.readyState === 'complete' && typeof render === 'function'")?
                .as_bool()
                == Some(true)
            {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let window = cdp.call("Runtime.evaluate", json!({"expression":"window"}))?;
        let rendered = cdp.call("Runtime.callFunctionOn", json!({
            "objectId":window["result"]["objectId"],
            "functionDeclaration":format!("function(xml, bundled) {{ {} return dipRender(xml, bundled); }}", include_str!("../assets/chromium-render.js")),
            "arguments":[{"value":xml},{"value":server.bundled}],
            "returnByValue":true
        }))?;
        ensure!(
            rendered.get("exceptionDetails").is_none(),
            "draw.io JavaScript error: {}",
            rendered["exceptionDetails"]
        );
        let bounds = loop {
            let result = cdp.eval("document.getElementById('LoadingComplete') ? JSON.parse(document.getElementById('LoadingComplete').getAttribute('bounds')) : null")?;
            if !result.is_null() {
                break result;
            }
            thread::sleep(Duration::from_millis(20));
        };
        let width = dimension(&bounds, "width", "x")?;
        let height = dimension(&bounds, "height", "y")?;
        resample::capture_size(width, height)?;
        cdp.call(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":height,"deviceScaleFactor":resample::SCALE,"mobile":false}),
        )?;
        cdp.call("Runtime.evaluate", json!({"expression":"document.fonts.ready.then(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))))", "awaitPromise":true}))?;
        cdp.eval("true")?;
        let screenshot = cdp.call("Page.captureScreenshot", json!({"format":"png","captureBeyondViewport":true,"clip":{"x":0,"y":0,"width":width,"height":height,"scale":1}}))?;
        let data = screenshot["data"]
            .as_str()
            .context("Chromium returned no PNG")?;
        ensure!(
            data.len() <= MAX_BYTES.div_ceil(3) * 4,
            "encoded PNG exceeds size limit"
        );
        let png = STANDARD
            .decode(data)
            .context("invalid Chromium PNG encoding")?;
        resample::half_png(&png, width, height, deadline).context("cannot resize Chromium PNG")
    })();
    result.with_context(|| format!("Chromium rendering failed: {}", log_excerpt(&mut log)))
}

fn remaining(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .context("Chromium rendering timed out")
}
fn dimension(bounds: &Value, size: &str, offset: &str) -> Result<u32> {
    // Desktop adds one pixel after rounding to keep scrollbars out of the export.
    let value = (bounds[size].as_f64().context("invalid render bounds")?
        + bounds[offset].as_f64().context("invalid render bounds")?)
    .ceil()
        + 1.0;
    ensure!(
        value.is_finite() && value >= 2.0 && value <= (MAX_BYTES / 4) as f64,
        "invalid or oversized render bounds"
    );
    Ok(value as u32)
}

struct Cdp {
    socket: WebSocket<TcpStream>,
    next_id: u64,
    session: Option<String>,
    deadline: Instant,
    origin: String,
    allow_network: bool,
    transferred: u64,
    log: fs::File,
}
impl Cdp {
    fn send(&mut self, method: &str, params: Value) -> Result<u64> {
        let timeout = remaining(self.deadline)?;
        self.socket.get_mut().set_read_timeout(Some(timeout))?;
        self.socket.get_mut().set_write_timeout(Some(timeout))?;
        self.next_id += 1;
        let mut message = json!({"id":self.next_id,"method":method,"params":params});
        if let Some(session) = &self.session {
            message["sessionId"] = json!(session);
        }
        self.socket.send(Message::text(message.to_string()))?;
        Ok(self.next_id)
    }
    fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.send(method, params)?;
        loop {
            ensure!(
                self.log.metadata()?.len() <= MAX_BYTES as u64,
                "Chromium log exceeds 64 MiB limit"
            );
            self.socket.get_mut().set_read_timeout(Some(
                remaining(self.deadline)?.min(Duration::from_millis(100)),
            ))?;
            let message = match self.socket.read() {
                Err(tungstenite::Error::Io(e))
                    if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
                {
                    remaining(self.deadline)
                        .with_context(|| format!("Chromium timed out during {method}"))?;
                    continue;
                }
                other => other.context("Chromium CDP connection closed")?,
            };
            let Message::Text(message) = message else {
                continue;
            };
            let value: Value = serde_json::from_str(&message).context("invalid CDP response")?;
            if value["id"].as_u64() == Some(id) {
                ensure!(
                    value.get("error").is_none(),
                    "Chromium {method} failed: {}",
                    value["error"]
                );
                return Ok(value["result"].clone());
            }
            self.event(&value)?;
        }
    }
    fn event(&mut self, event: &Value) -> Result<()> {
        let params = &event["params"];
        match event["method"].as_str().unwrap_or("") {
            "Fetch.requestPaused" => {
                let url = params["request"]["url"].as_str().unwrap_or("");
                let local = url.starts_with(&format!("{}/", self.origin));
                let allowed = local
                    || url.starts_with("data:")
                    || url.starts_with("blob:")
                    || (self.allow_network
                        && (url.starts_with("https://") || url.starts_with("http://")));
                if allowed {
                    self.send(
                        "Fetch.continueRequest",
                        json!({"requestId":params["requestId"]}),
                    )?;
                } else {
                    self.send(
                        "Fetch.failRequest",
                        json!({"requestId":params["requestId"],"errorReason":"BlockedByClient"}),
                    )?;
                    bail!(
                        "external resource blocked; use --allow-network to permit HTTP(S) images/fonts: {}",
                        short(url)
                    );
                }
            }
            "Network.responseReceived" => {
                let response = &params["response"];
                let url = response["url"].as_str().unwrap_or("");
                ensure!(
                    response["status"].as_f64().unwrap_or(200.0) < 400.0
                        || url.ends_with("/favicon.ico"),
                    "draw.io resource failed ({}): {}; check DIP_DRAWIO_WEB_PATH or --allow-network",
                    response["status"],
                    short(url)
                );
            }
            "Network.loadingFailed" => {
                bail!(
                    "draw.io resource failed: {}; check assets and --allow-network",
                    params["errorText"]
                );
            }
            "Network.dataReceived" => {
                self.transferred = self
                    .transferred
                    .saturating_add(params["dataLength"].as_u64().unwrap_or(0));
                ensure!(
                    self.transferred <= MAX_BYTES as u64,
                    "browser resource data exceeds 64 MiB limit"
                );
            }
            _ => {}
        }
        Ok(())
    }
    fn eval(&mut self, expression: &str) -> Result<Value> {
        let result = self.call("Runtime.evaluate", json!({"expression":format!("(() => {{ if (window.dipErrors?.length) throw Error(window.dipErrors.join('; ')); return ({expression}); }})()"),"returnByValue":true}))?;
        ensure!(
            result.get("exceptionDetails").is_none(),
            "draw.io JavaScript error: {}",
            result["exceptionDetails"]
        );
        Ok(result["result"]["value"].clone())
    }
}
fn short(value: &str) -> String {
    value.chars().take(256).collect()
}

// Closing this job also terminates Chromium's subprocesses on Windows.
#[cfg(windows)]
struct WindowsJob(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl WindowsJob {
    fn assign(child: &std::process::Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            ensure!(
                !handle.is_null(),
                "cannot create Chromium job: {}",
                std::io::Error::last_os_error()
            );
            let job = Self(handle);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            ensure!(
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of_val(&info) as u32
                ) != 0,
                "cannot configure Chromium job: {}",
                std::io::Error::last_os_error()
            );
            ensure!(
                AssignProcessToJobObject(handle, child.as_raw_handle()) != 0,
                "cannot assign Chromium job: {}",
                std::io::Error::last_os_error()
            );
            Ok(job)
        }
    }
}
#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
