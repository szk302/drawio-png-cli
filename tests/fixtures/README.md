# Compatibility fixtures

These fixtures are original test data, not copied diagram assets.

- `model.xml`: one graph with Japanese text, escaped HTML, a wrapped cell, and an unknown attribute.
- `mixed-pages.xml`: a compressed first page and an uncompressed second page, with document/page extension attributes and an extension element.
- `text.drawio.png`, `zlib.drawio.png`, `double.drawio.png`: the mixed-page document in URL-encoded `tEXt`, zlib `zTXt`, and double URL-encoded `tEXt`.
- `legacy.drawio.png`: the single model stored as raw DEFLATE `zTXt` with the `mxGraphModel` key.
- `plain.png`: transparent RGBA pixel plus an unrelated `Author` text chunk.

The metadata PNG fixtures above were constructed independently of `dip`, using Python standard-library `struct`, `zlib`, `base64`, and `urllib.parse`. Chunk CRCs use `zlib.crc32`; scanlines contain one PNG filter byte followed by four zero RGBA bytes. The fixtures are committed so tests need no Python runtime.

## VS Code PNG reference images

- `geometry-vscode.png`: 102x42 PNG produced by the VS Code extension's `xmlpng`
  save path, using a rectangle at 10,20 with size 100x40, fill `#dae8fc`,
  stroke `#6c8ebf`, and no label. The exact XML is inline in
  `real_chromium_renders_first_page_and_preserves_xml`.
- `geometry-options-vscode.png`: the same rectangle in an `mxfile` with
  `scale="2" border="10"`, exported as 244x124.

These images were generated independently of dip using the extension's Webview
HTML at `79500e6d467a95906a5f03680627c8f26ad3a0af`, with draw.io assets pinned at
`f3abfe0f082c18f7b4fee8a34c2d07b1987687fd`, in Chromium 153.0.8010.47 on Linux
ARM64. A test bridge supplied the VS Code message API; VS Code itself was not
running. DPR 1 and 2 produced identical pixels. Both fixtures retain the PNG's
embedded XML, but tests compare decoded RGBA only. See the
[comparison record](../../docs/vscode-rendering.md) for details.

## Rendering and resizing reference images

- `geometry-desktop.png`: the 104x44 output of draw.io Desktop 31.4.5 / Electron
  44.2.0, on Linux ARM64 with a 1x Xvfb display. The original rectangle input is
  inline in `real_chromium_modes_select_distinct_outputs_and_preserve_xml`: position 10,20,
  size 100x40, fill `#dae8fc`, stroke `#6c8ebf`, no label. Stored as RGBA8 with
  unchanged pixels and no metadata for comparison with Chromium output.
- `resize-input.png`: original 16x12 RGBA8 synthetic image, independently encoded
  with Python `struct` and `zlib`. Pixel `(x,y)` is
  `((37*x+13*y)%256, (17*x+41*y)%256, (29*x+23*y)%256, alpha[(x+y)%7])`, where
  `alpha = [0, 1, 31, 127, 128, 200, 255]`. This covers transparency, hidden RGB,
  partially transparent colors and filter clipping at the image edges.
- `resize-hamming1.png`: the 8x6 result produced from `resize-input.png` by
  Electron 44.2.0's nativeImage, independently of dip's Rust implementation:
  `nativeImage.createFromPath(input).resize({width:8,height:6,quality:'good'}).toPNG()`.
  Chromium 152's `good` and `better` paths use Hamming1. Its default `best` path
  uses Lanczos3 and is not the reference used by this test.

Desktop's actual `capturePage()` on the 1x display had already reduced the DPR 2
rendering to 104x44 before the application's final `resize()` call. Captured and
final PNGs were identical; applying nativeImage's Hamming1 resize to the full
208x88 Chromium capture produced identical RGBA pixels. See
[rendering notes](../../docs/rendering.md) for scope and limits.

The formats follow the locally inspected draw.io sources:

- drawio `744cb5420`: `Editor.extractGraphModelFromPng`, `Editor.writeGraphModelToPng`, `Graph.compress`, and `Graph.decompress`.
- drawio-desktop `60ec92a`: Desktop PNG export and CLI arguments.
- drawio-exporter `68aa3df`: Desktop invocation and compatibility-test organization.

Reference attribution and the drawio-exporter license text are recorded in
[THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).
