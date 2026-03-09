import './styles/main.css';
import { initNavigation } from './navigate';
import { initScrollReveal } from './scroll';
import { initTheme } from './theme';
import { renderMath } from './math';

initTheme();

document.fonts.ready.then(() => {
    document.body.classList.add('loaded');
    initScrollReveal();
    initNavigation();
    renderMath();
});
