const delimiters = [
    { left: '$$', right: '$$', display: true },
    { left: '$', right: '$', display: false },
];

function tryRender(root: Element | Document) {
    const render = (window as any).renderMathInElement;
    if (render) {
        render(root, { delimiters });
        return true;
    }
    return false;
}

export function renderMath(root: Element | Document = document.body) {
    if (!tryRender(root)) {
        // katex not loaded yet, wait for it
        const check = setInterval(() => {
            if (tryRender(root)) clearInterval(check);
        }, 50);
        setTimeout(() => clearInterval(check), 5000);
    }
}
