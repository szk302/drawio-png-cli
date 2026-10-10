#!/usr/bin/env python3
"""Regenerate the draw.io shape catalog from a local upstream checkout (maintainer only).

Usage: shape_catalog.py /path/to/drawio-checkout

Runs the sidebar of the pinned vscode-mode draw.io (26.0.2) in headless Chrome
(DIP_CHROME_PATH, CHROME_PATH, or google-chrome/chromium on PATH) and writes
assets/shapes/catalog.json.gz, which dip embeds as the built-in `drawio/*`
libraries of `dip library` and `dip insert`.

Entries whose cells embed image data (`data:image/...`) are left out, so the
catalog carries only cells, styles and references, never icon artwork.
"""
import gzip
import html
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
REVISION = '96a916a337d13fc8bf622c8a67d422bd284eabe5'  # draw.io 26.0.2, as in vendor_drawio.py
BUNDLE = ROOT / 'assets/shapes/catalog.json.gz'


def chrome():
    for name in ('DIP_CHROME_PATH', 'CHROME_PATH'):
        if os.environ.get(name):
            return os.environ[name]
    for name in ('google-chrome', 'chromium', 'chromium-browser'):
        if shutil.which(name):
            return shutil.which(name)
    sys.exit('Chrome not found; set DIP_CHROME_PATH')


def palettes(checkout):
    subprocess.check_call(['git', '-C', str(checkout), 'cat-file', '-e', REVISION + '^{commit}'])
    with tempfile.TemporaryDirectory() as work:
        # Read the pinned Git objects, never uncommitted checkout contents.
        archive = subprocess.check_output(
            ['git', '-C', str(checkout), 'archive', REVISION, 'src/main/webapp'])
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(work, filter='data')
        webapp = Path(work, 'src/main/webapp')
        shutil.copy(Path(__file__).with_name('shape_catalog.html'), webapp / 'catalog.html')
        dom = subprocess.run(
            [chrome(), '--headless=new', '--disable-gpu', '--no-sandbox', '--no-first-run',
             '--disable-background-networking', '--disable-component-update', '--disable-sync',
             '--user-data-dir=' + str(Path(work, 'profile')), '--allow-file-access-from-files',
             '--dump-dom', (webapp / 'catalog.html').as_uri()],
            check=True, capture_output=True, text=True, timeout=300).stdout
    match = re.search(r'<pre id="out">(.*?)</pre>', dom, re.DOTALL)
    if not match or not match.group(1):
        sys.exit('the catalog page produced no output')
    result = json.loads(html.unescape(match.group(1)))
    for error in result['errors']:
        print('note: ' + error, file=sys.stderr)
    return result['palettes']


def slug(palette_id):
    text = re.sub(r'(?<=[a-z0-9])(?=[A-Z])', '-', palette_id)
    return re.sub(r'[^a-z0-9]+', '-', text.lower()).strip('-')


def text(value):
    return re.sub(r'\s+', ' ', value or '').strip()


def size(value):
    return value if isinstance(value, (int, float)) and value > 0 else 0


def main():
    if len(sys.argv) != 2:
        sys.exit('Usage: shape_catalog.py /path/to/drawio-checkout')
    catalog, seen = [], set()
    for palette in palettes(Path(sys.argv[1])):
        # A palette built without its family's arguments has broken styles and sizes.
        if any(re.search(r'undefined|="NaN"', e['xml']) for e in palette['entries']):
            print(f'note: skipped {palette["id"]}: built without its arguments', file=sys.stderr)
            continue
        entries = [{'title': text(e['title']), 'w': size(e.get('w')), 'h': size(e.get('h')), 'xml': e['xml']}
                   for e in palette['entries'] if 'data:image' not in e['xml']]
        if not entries:
            continue
        name = slug(palette['id'])
        assert name not in seen, name
        seen.add(name)
        catalog.append({'name': name, 'title': text(palette['title']), 'entries': entries})
    # Name order, independent of the order the sidebar functions ran in.
    catalog.sort(key=lambda p: p['name'])
    names = {p['name'] for p in catalog}
    for required in ('aws4-compute', 'azure2-compute', 'gcp2-zones', 'kubernetes'):
        assert required in names, required
    data = json.dumps({'revision': REVISION, 'palettes': catalog},
                      ensure_ascii=False, separators=(',', ':'))
    BUNDLE.parent.mkdir(exist_ok=True)
    BUNDLE.write_bytes(gzip.compress(data.encode(), compresslevel=9, mtime=0))
    print(f'{len(catalog)} palettes, {sum(len(p["entries"]) for p in catalog)} shapes, '
          f'{BUNDLE.stat().st_size} bytes')


if __name__ == '__main__':
    main()
