// Called with JSON arguments through CDP, never XML interpolated as source code.
function dipRender(xml, bundled) {
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
    render({xml, format: 'png', from: 0, to: 0, scale: 1, border: 0, theme: 'light'});
}
