// Called with JSON arguments through CDP, never XML interpolated as source code.
async function dipRender(xml, bundled, mode) {
    if (bundled) mxStencilRegistry.dynamicLoading = false;
    const doc = mxUtils.parseXml(xml);
    if (doc.getElementsByTagName('parsererror').length) throw Error('Invalid XML');
    const model = doc.documentElement.nodeName === 'mxGraphModel'
        ? doc.documentElement : doc.querySelector('diagram > mxGraphModel');
    if (!model) throw Error('Expected an uncompressed mxGraphModel');
    if (bundled && (model.getAttribute('math') === '1' ||
        model.querySelector('[layout]') || model.hasAttribute('layout'))) {
        throw Error('Math and automatic layout require local draw.io assets (DIP_DRAWIO_WEB_PATH)');
    }
    const original = mxCellRenderer.prototype.createShape;
    mxCellRenderer.prototype.createShape = function(state) {
        const name = state.style[mxConstants.STYLE_SHAPE];
        if (name && !mxCellRenderer.defaultShapes[name] && !mxStencilRegistry.getStencil(name)) {
            throw Error('Unsupported shape: ' + name + '; provide assets with DIP_DRAWIO_WEB_PATH');
        }
        return original.apply(this, arguments);
    };
    if (mode !== 'vscode') {
        render({xml, format: 'png', from: 0, to: 0, scale: 1, border: 0, theme: 'light'});
        return;
    }
    const file = doc.documentElement;
    const option = (name, fallback) => {
        const value = file.nodeName === 'mxfile' ? Number.parseFloat(file.getAttribute(name)) : NaN;
        return Number.isNaN(value) ? fallback : value;
    };
    const scale = option('scale', 1), border = option('border', 0);
    if (!Number.isFinite(scale) || scale <= 0 || !Number.isFinite(border) || border < 0) {
        throw Error('Invalid PNG scale or border');
    }
    const container = document.createElement('div');
    document.body.appendChild(container);
    const graph = new Graph(container);
    const editor = new Editor(false, null, null, graph);
    graph.setEnabled(false);
    const page = model.parentNode;
    const getGlobalVariable = graph.getGlobalVariable;
    graph.getGlobalVariable = function(name) {
        if (name === 'page') return page.nodeName === 'diagram' ? page.getAttribute('name') : '';
        if (name === 'pagenumber') return 1;
        return getGlobalVariable.apply(this, arguments);
    };
    // mxCodec resolves IDs through the owner document. Separate the page so
    // identical cell IDs on later pages cannot affect the first page's graph.
    editor.setGraphXml(mxUtils.parseXml(mxUtils.getXml(model)).documentElement);
    await document.fonts.ready;
    graph.refresh();
    // Guard both the intermediate SVG image and final Canvas before allocation.
    // Unlike the editor, the CLI rejects oversized exports rather than reducing scale.
    const checkSize = (w, h) => {
        if (!Number.isFinite(w) || !Number.isFinite(h) || w < 1 || h < 1 ||
            w > 16384 || h > 16384 || w * h * 4 > 64 * 1024 * 1024) {
            throw Error('Canvas image exceeds 64 MiB limit or maximum dimensions');
        }
    };
    const getSvg = graph.getSvg;
    graph.getSvg = function() {
        const svg = getSvg.apply(this, arguments);
        const w = parseFloat(svg.getAttribute('width')), h = parseFloat(svg.getAttribute('height'));
        checkSize(w, h);
        checkSize(Math.ceil(w * scale), Math.ceil(h * scale));
        return svg;
    };
    const canvas = await new Promise((resolve, reject) => {
        editor.exportToCanvas(resolve, null, null, null,
            error => reject(Error(error?.message || 'SVG to Canvas export failed')),
            // Resolve URLs directly; do not send images to draw.io's public proxy.
            null, null, scale, null, null, new mxUrlConverter(), graph, border);
    });
    checkSize(canvas.width, canvas.height);
    const uri = canvas.toDataURL('image/png');
    if (!uri.startsWith('data:image/png;base64,')) throw Error('Canvas returned no PNG');
    return uri.slice('data:image/png;base64,'.length);
}
