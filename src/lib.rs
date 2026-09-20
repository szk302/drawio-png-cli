pub mod chromium;
mod chromium_assets;
pub mod document;
pub mod fonts;
pub mod png_data;
pub mod render;
pub mod storage;

pub const MAX_BYTES: usize = 64 * 1024 * 1024;

/// Notices for the JavaScript renderer embedded in the binary.
pub const BUNDLED_LICENSES: &str = concat!(
    include_str!("../assets/drawio/README.md"),
    "\n\n--- DOMPurify-Apache-2.0.txt ---\n",
    include_str!("../assets/drawio/licenses/DOMPurify-Apache-2.0.txt"),
    "\n\n--- drawio-Apache-2.0.txt ---\n",
    include_str!("../assets/drawio/licenses/drawio-Apache-2.0.txt"),
    "\n\n--- hachure-fill-MIT.txt ---\n",
    include_str!("../assets/drawio/licenses/hachure-fill-MIT.txt"),
    "\n\n--- pako-MIT.txt ---\n",
    include_str!("../assets/drawio/licenses/pako-MIT.txt"),
    "\n\n--- pako-zlib.txt ---\n",
    include_str!("../assets/drawio/licenses/pako-zlib.txt"),
    "\n\n--- path-data-parser-MIT.txt ---\n",
    include_str!("../assets/drawio/licenses/path-data-parser-MIT.txt"),
    "\n\n--- points-on-curve-MIT.txt ---\n",
    include_str!("../assets/drawio/licenses/points-on-curve-MIT.txt"),
    "\n\n--- points-on-path-MIT.txt ---\n",
    include_str!("../assets/drawio/licenses/points-on-path-MIT.txt"),
    "\n\n--- roughjs-MIT.txt ---\n",
    include_str!("../assets/drawio/licenses/roughjs-MIT.txt"),
    "\n\n--- spin-MIT.txt ---\n",
    include_str!("../assets/drawio/licenses/spin-MIT.txt"),
);
