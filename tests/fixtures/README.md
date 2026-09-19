# Compatibility fixtures

These fixtures are original test data, not copied diagram assets.

- `model.xml`: one graph with Japanese text, escaped HTML, a wrapped cell, and an unknown attribute.
- `mixed-pages.xml`: a compressed first page and an uncompressed second page, with document/page extension attributes and an extension element.
- `text.drawio.png`, `zlib.drawio.png`, `double.drawio.png`: the mixed-page document in URL-encoded `tEXt`, zlib `zTXt`, and double URL-encoded `tEXt`.
- `legacy.drawio.png`: the single model stored as raw DEFLATE `zTXt` with the `mxGraphModel` key.
- `plain.png`: transparent RGBA pixel plus an unrelated `Author` text chunk.

PNG fixtures were constructed independently of `dip`, using Python standard-library `struct`, `zlib`, `base64`, and `urllib.parse`. Chunk CRCs use `zlib.crc32`; scanlines contain one PNG filter byte followed by four zero RGBA bytes. The fixtures are committed so tests need no Python runtime.

The formats follow the locally inspected draw.io sources:

- drawio `744cb5420`: `Editor.extractGraphModelFromPng`, `Editor.writeGraphModelToPng`, `Graph.compress`, and `Graph.decompress`.
- drawio-desktop `60ec92a`: Desktop PNG export and CLI arguments.
- drawio-exporter `68aa3df`: Desktop invocation and compatibility-test organization.

Reference attribution and the drawio-exporter license text are recorded in
[THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).
