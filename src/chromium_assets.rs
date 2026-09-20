use crate::storage;
use anyhow::{Context, Result, ensure};
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tiny_http::{Header, Method, Response, Server};

pub(crate) const ASSETS: &[(&str, &[u8])] = &[
    (
        "js/viewer.min.js",
        include_bytes!("../assets/drawio/js_viewer.min.js.gz"),
    ),
    (
        "js/export-init.js",
        include_bytes!("../assets/drawio/js_export-init.js.gz"),
    ),
    (
        "js/export.js",
        include_bytes!("../assets/drawio/js_export.js.gz"),
    ),
    (
        "mxgraph/css/common.css",
        include_bytes!("../assets/drawio/mxgraph_css_common.css.gz"),
    ),
];

pub(crate) struct AssetServer {
    pub(crate) origin: String,
    pub(crate) bundled: bool,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

fn local_file(root: &Path, url: &str) -> Result<PathBuf> {
    let decoded = crate::document::percent_decode(url.split('?').next().unwrap_or(url))?;
    let relative = decoded.strip_prefix('/').context("invalid asset path")?;
    ensure!(!relative.contains('\\'), "invalid asset path");
    ensure!(
        Path::new(relative)
            .components()
            .all(|c| matches!(c, Component::Normal(_))),
        "invalid asset path"
    );
    let path = root.join(relative).canonicalize()?;
    ensure!(
        path.starts_with(root) && path.is_file(),
        "asset outside web root"
    );
    Ok(path)
}

impl AssetServer {
    pub(crate) fn start(root: Option<PathBuf>, allow_network: bool) -> Result<Self> {
        let root = root
            .map(|p| p.canonicalize().context("invalid DIP_DRAWIO_WEB_PATH"))
            .transpose()?;
        if let Some(root) = &root {
            ensure!(
                root.is_dir() && root.join("export3.html").is_file(),
                "DIP_DRAWIO_WEB_PATH must contain export3.html"
            );
        }
        let bundled = root.is_none();
        let mut assets = HashMap::new();
        if bundled {
            for &(name, bytes) in ASSETS {
                let bytes = storage::read_limited(flate2::read::GzDecoder::new(bytes))?;
                assets.insert(format!("/{name}"), bytes);
            }
            assets.insert(
                "/export3.html".into(),
                include_bytes!("../assets/chromium.html").to_vec(),
            );
        }
        let server = Server::http("127.0.0.1:0")
            .map_err(|e| anyhow::anyhow!("cannot start asset server: {e}"))?;
        let origin = format!("http://{}", server.server_addr());
        // CSP also covers WebSockets, workers and requests not intercepted by CDP.
        let sources = if allow_network {
            "'self' data: blob: http: https:"
        } else {
            "'self' data: blob:"
        };
        let csp = format!(
            "default-src {sources}; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src {sources} 'unsafe-inline'; object-src 'none'; frame-src 'none'; worker-src 'none'; base-uri 'self'"
        );
        let stop = Arc::new(AtomicBool::new(false));
        let signal = stop.clone();
        let worker = thread::spawn(move || {
            while !signal.load(Ordering::Relaxed) {
                let Ok(Some(request)) = server.recv_timeout(Duration::from_millis(50)) else {
                    continue;
                };
                let path = request.url().split('?').next().unwrap_or(request.url());
                let bytes = if request.method() != &Method::Get {
                    None
                } else if let Some(root) = &root {
                    local_file(root, path).and_then(|p| storage::read(&p)).ok()
                } else {
                    assets.get(path).cloned()
                };
                let (bytes, status) = bytes
                    .map(|b| (b, 200))
                    .unwrap_or((b"asset not found".to_vec(), 404));
                let mime = match Path::new(path).extension().and_then(|s| s.to_str()) {
                    Some("js") => "application/javascript",
                    Some("css") => "text/css",
                    Some("html") => "text/html; charset=utf-8",
                    Some("xml") => "application/xml",
                    Some("svg") => "image/svg+xml",
                    Some("png") => "image/png",
                    Some("woff2") => "font/woff2",
                    _ => "application/octet-stream",
                };
                let mut response = Response::from_data(bytes).with_status_code(status);
                response.add_header(Header::from_bytes("Content-Type", mime).unwrap());
                response.add_header(
                    Header::from_bytes("Content-Security-Policy", csp.as_str()).unwrap(),
                );
                response.add_header(Header::from_bytes("Cache-Control", "no-store").unwrap());
                let _ = request.respond(response);
            }
        });
        Ok(Self {
            origin,
            bundled,
            stop,
            worker: Some(worker),
        })
    }
}
impl Drop for AssetServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn web_root_rejects_traversal() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("ok.js"), "ok").unwrap();
        assert!(local_file(root.path(), "/ok.js").is_ok());
        for path in ["/../secret", "/%2e%2e/secret", "/%5csecret", "/C:/secret"] {
            assert!(local_file(root.path(), path).is_err());
        }
    }
}
