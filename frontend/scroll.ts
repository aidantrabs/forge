export function initScrollReveal() {
    const elements = document.querySelectorAll('.animate-fade-in-up');
    if (!elements.length) return;

    const observer = new IntersectionObserver(
        (entries) => {
            for (const entry of entries) {
                if (entry.isIntersecting) {
                    entry.target.classList.add('is-visible');
                    observer.unobserve(entry.target);
                }
            }
        },
        { threshold: 0.1, rootMargin: '0px 0px -50px 0px' }
    );

    for (const el of elements) {
        observer.observe(el);
    }
}
