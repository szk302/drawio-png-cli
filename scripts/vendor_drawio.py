#!/usr/bin/env python3
"""Refresh pinned browser assets from a local upstream checkout (maintainer only)."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys

REVISION = '744cb5420fdf126efd7a09b1d7082ca3e12c0841'
FILES = ['js/viewer.min.js', 'js/export-init.js', 'js/export.js', 'mxgraph/css/common.css']
out = Path(__file__).resolve().parents[1] / 'assets/drawio'
if len(sys.argv) != 2:
    sys.exit('Usage: vendor_drawio.py --check | /path/to/drawio-checkout')
if sys.argv[1] == '--check':
    manifest = json.loads((out / 'manifest.json').read_text())
    assert manifest['revision'] == REVISION
    assert [entry['source'] for entry in manifest['assets']] == ['src/main/webapp/' + name for name in FILES]
    for entry in manifest['assets']:
        data = gzip.decompress((out / entry['file']).read_bytes())
        assert hashlib.sha256(data).hexdigest() == entry['sha256'], entry['file']
    print('Bundled draw.io asset hashes verified')
    sys.exit(0)
root = Path(sys.argv[1])
assert subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip() == REVISION
entries = []
for name in FILES:
    # Read the pinned Git object, never uncommitted checkout contents.
    source = 'src/main/webapp/' + name
    data = subprocess.check_output(['git', '-C', str(root), 'show', REVISION + ':' + source])
    target = name.replace('/', '_') + '.gz'
    (out / target).write_bytes(gzip.compress(data, mtime=0))
    entries.append({'source': source, 'file': target, 'sha256': hashlib.sha256(data).hexdigest()})
(out / 'manifest.json').write_text(json.dumps({'revision': REVISION, 'assets': entries}, indent=2) + '\n')
(out / 'licenses/drawio-Apache-2.0.txt').write_bytes(subprocess.check_output(['git', '-C', str(root), 'show', REVISION + ':LICENSE']))
