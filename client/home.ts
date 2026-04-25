import { SVG_COMMENT, SVG_LIKE } from "./assets";



export class HomeNav extends HTMLElement {



    static get observedAttributes() {
        return ['data-value'];
    }


    attributeChangedCallback(name: string, oldValue: string, newValue: string) {
        if (oldValue !== newValue) {
            this.render();
        }
    }

    connectedCallback() {
        this.render();
    }

    render() {
        const val = this.getAttribute('data-value') || 'Пусто';
        this.innerHTML = `<div>Значение: ${val}</div>`;
    }
}



export class Feed extends HTMLElement {
    constructor() {
        super();
    }

    connectedCallback() {
        this.render();
        this.fetchData();
    }

    render(content?: string) {
        this.innerHTML = `
        <div id="container" class="feed">
        ${content ? content : '<div class="loader-wrapper"><div class="loader"></div></div>'}
        </div>
        `;
    }

    async fetchData() {
        const url = this.getAttribute('src') || "/api/getfeed";

        try {
            const response = await fetch(url);
            const data = await response.json();

            const posts = Array.isArray(data)
            ? data
            : (data.posts && Array.isArray(data.posts) ? data.posts : []);

            if (posts.length === 0) {
                this.render(`<span>Лента пуста или пришел неверный формат данных</span>`);
                return;
            }

            const feedHtml = posts.map((post: any) => {
                let files: string[] = [];
                try { files = JSON.parse(post.files || '[]'); } catch { /* ignore */ }

                const isImageUrl = (url: string) =>
                url.startsWith('/api/files/') &&
                /\.(jpg|jpeg|png|gif|webp)(\?.*)?$/i.test(url);

                const imagesHtml = files
                .filter(isImageUrl)
                .map(url => `<img src="${url}" alt="" class="post-img" loading="lazy">`)
                .join('');

                return `
                <div class="post-card">
                <div class="post-text">
                <h2>${this.escapeHtml(post.title || 'Без названия')}</h2>
                <p class="post-text-body">${this.escapeHtml(post.content || '')}</p>
                <div class="post-info">
                <img src="${SVG_COMMENT}" alt="Комменты">
                <p>${this.escapeHtml(String(post.comment_count ?? '0'))}</p>
                <img src="${SVG_LIKE}" alt="Лайки">
                <p>${this.escapeHtml(String(post.like_count ?? '0'))}</p>
                </div>
                </div>
                ${imagesHtml}
                </div>
                `;
            }).join('');

            this.render(feedHtml);
        } catch (err) {
            this.render(`<span style="color: red;">Error: ${err}</span>`);
        }
    }

    private escapeHtml(unsafe: string) {
        return unsafe
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
    }
}
