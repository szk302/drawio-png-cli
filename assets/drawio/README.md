# Bundled draw.io renderer

Upstream: https://github.com/jgraph/drawio
Revision: `f3abfe0f082c18f7b4fee8a34c2d07b1987687fd`.

`manifest.json` identifies the four unmodified upstream assets and SHA-256
hashes of their decompressed bytes. They are stored with deterministic gzip
headers and embedded into `dip`. `chromium.html`, `chromium-init.js`, and
`chromium-render.js` in the parent directory are original MIT-licensed dip
integration code; they configure local URLs, disable optional bundled math,
check unsupported shapes, and select `Editor.exportToCanvas()` for the `vscode`
mode or `render()` for the `raw` and `desktop` capture modes.
The revision matches the draw.io submodule pinned by the inspected
[`hediet.vscode-drawio`](https://github.com/hediet/vscode-drawio) extension
(`79500e6d467a95906a5f03680627c8f26ad3a0af`). Its reference-only use is documented
in [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). No upstream source is edited.

## Included works

- draw.io/mxGraph: Apache-2.0. Copyright JGraph Holdings Ltd and draw.io AG;
  individual years and notices remain in the upstream files.
- spin.js 2.0.0: MIT, Copyright (c) 2011-2014 Felix Gnass.
- DOMPurify 3.4.15: Apache-2.0 (selected from upstream's dual licensing),
  Copyright (c) Cure53 and other contributors.
- pako 2.2.0: MIT AND Zlib, Vitaly Puzrin, Andrei Tuputcyn/Tupitsin,
  Jean-loup Gailly and Mark Adler.
- Rough.js 4.6.6 and its hachure-fill, path-data-parser, points-on-path,
  points-on-curve components: MIT, Copyright (c) 2019/2020/2022 Preet Shihn.
  All upstream copyright statements are retained in `licenses/`.

The viewer composition was checked against `etc/build/build.xml`: the nonstatic
viewer contains the basic mxGraph/editor/viewer code and the libraries above.
It does not concatenate `shapes-14-6-5.min.js`. This package excludes `app.min.js`,
`viewer-static.min.js`, `extensions.min.js`, the additional `shapes/`, `stencils/`,
`img/` packs, their compiled derivatives, templates, MathJax, Mermaid, ELK and
libavoid. References to optional asset paths in the viewer are not bundled packs.
The separately restricted icon/stencil packs are intentionally excluded; consult
upstream's `shapes/LICENSE`, `stencils/LICENSE`, and `img/LICENSE` before obtaining
those assets for use through `DIP_DRAWIO_WEB_PATH`.

## License provenance

Full license texts in `licenses/` come from:

- draw.io: the pinned repository's `LICENSE`.
- spin.js: https://raw.githubusercontent.com/fgnass/spin.js/2.0.0/LICENSE.txt
- DOMPurify: https://raw.githubusercontent.com/cure53/DOMPurify/3.4.15/LICENSE
- pako: https://raw.githubusercontent.com/nodeca/pako/2.2.0/LICENSE and
  https://raw.githubusercontent.com/nodeca/pako/2.2.0/lib/zlib/README
- Rough.js and its components: the `LICENSE` files in npm source tarballs for
  roughjs 4.6.6, hachure-fill 0.5.2, path-data-parser 0.1.0,
  points-on-path 0.2.1 and points-on-curve 0.2.0. Each is also covered by the
  corresponding MIT copyright notices here; the minified viewer does not
  preserve the exact transitive dependency lockfile used by upstream.

`dip licenses` prints these notices and full texts, so standalone binary users
can read them. Source and binary archives must also retain `LICENSE`,
`THIRD_PARTY_NOTICES.md`, this file, `licenses/`, and `../licenses/` (Chromium
resampling and Cargo dependency licenses).

## Maintenance

Run `python3 scripts/vendor_drawio.py /path/to/drawio-checkout` to refresh from
the pinned Git objects, then review the composition and licenses. To change
versions, update the revision deliberately and repeat that review.
Run `python3 scripts/vendor_drawio.py --check` to verify stored hashes without
network access or an upstream checkout. Python is only used for maintenance
and integrity checks, not building or running dip.
