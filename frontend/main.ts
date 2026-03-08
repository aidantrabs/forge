import './styles/main.css';
import { initScrollReveal } from './scroll';
import { initTheme } from './theme';

initTheme();

document.fonts.ready.then(() => {
    document.body.classList.add('loaded');
    initScrollReveal();
});
