import { bindPostCardClicks, POST_CARD_STYLES, PostCardData, renderPostCard } from './post-card';

// ----- SidebarNews -----

export class SidebarNews extends HTMLElement {
    private _boundRefresh = () => this.fetchData();

    connectedCallback() {
        this._injectStyles();
        this.render();
        this.fetchData();
        window.addEventListener('feed-refresh', this._boundRefresh);
    }

    disconnectedCallback() {
        window.removeEventListener('feed-refresh', this._boundRefresh);
    }

    render(content?: string) {
        this.innerHTML = `
        <div class="sn-wrap">
        ${content ?? '<div class="loader-wrapper"><div class="loader"></div></div>'}
        </div>`;
    }

    async fetchData() {
        try {
            const res = await fetch('/api/sidebar-news');
            if (!res.ok || res.status === 204) { this.render(''); return; }

            const post: PostCardData | null = await res.json();
            if (!post) { this.render(''); return; }

            const card = renderPostCard(post, 'sidebar');
            this.render(`
            <div class="sn-label">Новость</div>
            <div class="sn-card-wrap">${card}</div>`);

            const wrap = this.querySelector<HTMLElement>('.sn-card-wrap');
            if (wrap) bindPostCardClicks(wrap);
        } catch {
            this.render('');
        }
    }

    private _injectStyles() {
        if (document.getElementById('sidebar-news-styles')) return;
        const s       = document.createElement('style');
        s.id          = 'sidebar-news-styles';
        s.textContent = SIDEBAR_NEWS_STYLES;
        document.head.appendChild(s);
    }
}

// ----- styles -----

export const SIDEBAR_NEWS_STYLES = `
.sn-wrap {
    width: 240px;
    flex-shrink: 0;
    padding: 1em;
    box-sizing: border-box;
    position: sticky;
    top: 1em;
}

.sn-label {
    font-size: 0.72em;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--ltextc);
    margin-bottom: 0.5em;
}

.sn-card-wrap .pc-card {
    flex-direction: column;
    border-radius: 1em;
    padding: 0;
    overflow: hidden;
    margin: 0;
    cursor: pointer;
}

.sn-card-wrap .pc-card-images {
    order: -1;
    width: 100%;
}

.sn-card-wrap .pc-card-img {
    max-width: 100%;
    width: 100%;
    max-height: none;
    aspect-ratio: 16/9;
    object-fit: cover;
    border-radius: 0;
}

.sn-card-wrap .pc-card-body {
    padding: 0.75em;
}

.sn-card-wrap .pc-card-content {
    -webkit-line-clamp: 6;
    max-height: calc(1.5em * 6);
}
`;
