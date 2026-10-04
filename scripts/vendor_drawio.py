#!/usr/bin/env python3
"""Refresh pinned browser assets from a local upstream checkout (maintainer only)."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys

FILES = ['js/viewer.min.js', 'js/export-init.js', 'js/export.js', 'mxgraph/css/common.css']
ASSETS = Path(__file__).resolve().parents[1] / 'assets/drawio'
# raw/desktop follow draw.io Desktop 31.4.5; vscode follows the draw.io pinned by the
# stable hediet.vscode-drawio 1.9.0 release (tag v1.9.0), which is 26.0.2.
SETS = {
    ASSETS: 'f3abfe0f082c18f7b4fee8a34c2d07b1987687fd',
    ASSETS / 'vscode': '96a916a337d13fc8bf622c8a67d422bd284eabe5',
}
if len(sys.argv) != 2:
    sys.exit('Usage: vendor_drawio.py --check | /path/to/drawio-checkout')
if sys.argv[1] == '--check':
    for out, revision in SETS.items():
        manifest = json.loads((out / 'manifest.json').read_text())
        assert manifest['revision'] == revision, out
        assert [entry['source'] for entry in manifest['assets']] == ['src/main/webapp/' + name for name in FILES]
        for entry in manifest['assets']:
            data = gzip.decompress((out / entry['file']).read_bytes())
            assert hashlib.sha256(data).hexdigest() == entry['sha256'], out / entry['file']
    print('Bundled draw.io asset hashes verified')
    sys.exit(0)
root = Path(sys.argv[1])
for out, revision in SETS.items():
    subprocess.check_call(['git', '-C', str(root), 'cat-file', '-e', revision + '^{commit}'])
    out.mkdir(exist_ok=True)
    entries = []
    for name in FILES:
        # Read the pinned Git object, never uncommitted checkout contents.
        source = 'src/main/webapp/' + name
        data = subprocess.check_output(['git', '-C', str(root), 'show', revision + ':' + source])
        target = name.replace('/', '_') + '.gz'
        (out / target).write_bytes(gzip.compress(data, mtime=0))
        entries.append({'source': source, 'file': target, 'sha256': hashlib.sha256(data).hexdigest()})
    (out / 'manifest.json').write_text(json.dumps({'revision': revision, 'assets': entries}, indent=2) + '\n')
# Both revisions carry the same Apache-2.0 text; keep the one for the primary set.
(ASSETS / 'licenses/drawio-Apache-2.0.txt').write_bytes(
    subprocess.check_output(['git', '-C', str(root), 'show', SETS[ASSETS] + ':LICENSE']))
