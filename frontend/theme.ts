const STORAGE_KEY = 'theme';

function getPreferred(): 'dark' | 'light' {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'dark' || stored === 'light') return stored;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function setTheme(theme: 'dark' | 'light') {
    document.documentElement.classList.toggle('dark', theme === 'dark');
    localStorage.setItem(STORAGE_KEY, theme);
}

export function initTheme() {
    setTheme(getPreferred());

    const btn = document.querySelector('.pull-string');
    if (!btn) return;

    btn.addEventListener('click', () => {
        btn.classList.add('pulling');
        const next = document.documentElement.classList.contains('dark') ? 'light' : 'dark';
        setTheme(next);
        setTimeout(() => btn.classList.remove('pulling'), 350);
    });
}
