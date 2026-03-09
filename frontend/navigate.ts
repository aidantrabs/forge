import { initFilter } from './filter';
import { initScrollReveal } from './scroll';
import { renderMath } from './math';

function swapContent(html: string) {
    const parser = new DOMParser();
    const doc = parser.parseFromString(html, 'text/html');
    const newMain = doc.querySelector('main');
    const oldMain = document.querySelector('main');
    if (!newMain || !oldMain) return;

    document.title = doc.title;
    oldMain.classList.add('navigating');

    setTimeout(() => {
        oldMain.innerHTML = newMain.innerHTML;
        oldMain.classList.remove('navigating');
        window.scrollTo(0, 0);
        bindLinks();
        initScrollReveal();
        renderMath(oldMain);
        initFilter();
    }, 150);
}

function navigate(url: string) {
    fetch(url)
        .then((res) => res.text())
        .then((html) => {
            swapContent(html);
            history.pushState(null, '', url);
        });
}

function bindLinks() {
    document.querySelectorAll('a[data-navigate]').forEach((link) => {
        link.addEventListener('click', (e) => {
            const href = (link as HTMLAnchorElement).href;
            if (href && new URL(href).origin === location.origin) {
                e.preventDefault();
                navigate(href);
            }
        });
    });
}

export function initNavigation() {
    bindLinks();

    window.addEventListener('popstate', () => {
        navigate(location.href);
    });
}
