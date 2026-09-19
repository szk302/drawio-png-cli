# Third-party notices

This project's original code is licensed under the MIT License in [LICENSE](LICENSE).
Third-party works retain their own copyright notices and license terms.
This file documents implementation references; it is not an exhaustive license
inventory of Cargo dependencies.

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
included in this project's source package. dip calls a separately installed
Desktop executable; it does not bundle the application or draw.io rendering
assets. The test diagrams and PNG fixtures were generated for this project;
see [the fixture provenance](tests/fixtures/README.md).
