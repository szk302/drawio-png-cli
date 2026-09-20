# Third-party notices

This project's original code is licensed under the MIT License in [LICENSE](LICENSE).
Third-party works retain their own copyright notices and license terms.
This file documents implementation references; it is not an exhaustive license
inventory of Cargo dependencies.

## Chromium-compatible image resizing

`src/resample.rs` adapts the Hamming1 filter generation and fixed-point channel
rounding used by Chromium's `skia/ext/image_operations.cc` (Copyright 2012 The
Chromium Authors) and `skia/ext/convolver.{cc,h}` (Copyright 2011 The Chromium
Authors), reviewed at Chromium `152.0.7977.76`. The Rust implementation is limited
to 2:1 downsampling and uses bounded edge/interior kernel storage. It is licensed
under BSD-3-Clause; the full license is in
[`assets/licenses/chromium-BSD-3-Clause.txt`](assets/licenses/chromium-BSD-3-Clause.txt)
and is also printed by `dip licenses`.

The behavior reference is Electron `v44.2.0`
[`NativeImage::Resize`](https://github.com/electron/electron/blob/v44.2.0/shell/common/api/electron_api_native_image.cc),
whose `good` and `better` quality settings use Chromium's Hamming1 path. The
choice was verified against draw.io Desktop 31.4.5's actual capture output on a
1x Linux display; capture has already downsampled before Desktop calls resize.
No Electron code or runtime is bundled for this feature.

Retain this notice and the BSD license in source and binary distributions.

## drawio-exporter

- Project: https://github.com/rlespinasse/drawio-exporter
- Reviewed revision: `68aa3dfc52da4f70e0077aa569f6aa24d4a139d1`
- License: MIT
- Copyright: Copyright (c) 2021 Romain Lespinasse

The implementation consulted drawio-exporter for draw.io Desktop invocation,
standard executable locations, compressed-page decoding, and compatibility-test
organization. In particular, the references included
`src/drawio_exporter/core/drawio/drawio_desktop.rs`,
`src/drawio_exporter/core/drawio/mxfile.rs`, and its CLI tests.
These informed the separately written rendering, XML-processing, and test code
in this project. drawio-exporter is not a dependency and its source files and
fixtures are not bundled with dip.

The following upstream copyright and license text is retained in full as a
precautionary attribution for that implementation reference:

```text
MIT License

Copyright (c) 2021 Romain Lespinasse

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Other implementation references

- [drawio](https://github.com/jgraph/drawio), revision `744cb5420`:
  PNG metadata encoding/decoding and compressed diagram-page format.
  [Upstream license: Apache-2.0](https://github.com/jgraph/drawio/blob/744cb5420/LICENSE).
- [drawio-desktop](https://github.com/jgraph/drawio-desktop), revision `60ec92a`:
  Desktop export arguments and PNG export behavior.
  [Upstream license: Apache-2.0](https://github.com/jgraph/drawio-desktop/blob/60ec92a/LICENSE).

The local reference checkouts and Desktop test installation in `.tmp/` are not
included in this project's source package. dip calls a separately installed Desktop or Chromium executable.
The Chromium renderer bundles the limited subset described below. The test diagrams and PNG fixtures were generated for this project;
see [the fixture provenance](tests/fixtures/README.md).

## Bundled Chromium renderer assets

The Chromium renderer embeds an unmodified, gzip-compressed subset of draw.io
at revision `744cb5420fdf126efd7a09b1d7082ca3e12c0841`. This differs from the
reference-only use of the Desktop checkout described above.

See [the asset inventory](assets/drawio/README.md),
[the source/hash manifest](assets/drawio/manifest.json), and
[full license texts](assets/drawio/licenses/). The bundle includes Apache-2.0,
MIT and Zlib works; those assets are not relicensed under dip's MIT license.
Separately restricted additional icon/stencil packs and their compiled bundles
are excluded. All retained notices and license texts are accessible in standalone
binaries through `dip licenses`.

Binary archives must retain this file, `LICENSE`, `assets/drawio/README.md` and
`assets/drawio/licenses/`. No runtime or build-time asset download is performed.

The headless export integration also consulted jgraph/draw-image-export2's
`export.js` for its `render` / `LoadingComplete` protocol and screenshot sizing.
No server source or Node.js dependencies from that project are bundled.
