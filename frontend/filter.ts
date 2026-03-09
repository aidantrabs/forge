let activeTag: string | null = null;

function filterPosts() {
    const search = document.getElementById('post-search') as HTMLInputElement;
    const posts = document.querySelectorAll('.post-item');
    const noResults = document.getElementById('no-results');
    const countEl = document.getElementById('post-count');
    if (!search || !posts.length) return;

    const query = search.value.toLowerCase().trim();
    let visible = 0;

    posts.forEach((post) => {
        const el = post as HTMLElement;
        const title = el.dataset.title || '';
        const desc = el.dataset.description || '';
        const tags = el.dataset.tags || '';

        const matchesSearch = !query || title.includes(query) || desc.includes(query) || tags.includes(query);
        const matchesTag = !activeTag || tags.includes(activeTag);

        if (matchesSearch && matchesTag) {
            el.style.display = '';
            visible++;
        } else {
            el.style.display = 'none';
        }
    });

    if (noResults) noResults.classList.toggle('hidden', visible > 0);
    if (countEl) countEl.textContent = `${visible} post${visible !== 1 ? 's' : ''}`;
}

export function initFilter() {
    const search = document.getElementById('post-search') as HTMLInputElement;
    if (!search) return;

    search.addEventListener('input', filterPosts);

    document.querySelectorAll('.tag-filter').forEach((btn) => {
        btn.addEventListener('click', () => {
            const tag = (btn as HTMLElement).dataset.tag || '';
            if (activeTag === tag) {
                activeTag = null;
                btn.classList.remove('tag-filter--active');
            } else {
                document.querySelectorAll('.tag-filter').forEach((b) => b.classList.remove('tag-filter--active'));
                activeTag = tag;
                btn.classList.add('tag-filter--active');
            }
            filterPosts();
        });
    });
}
