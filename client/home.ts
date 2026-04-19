import { LOGO, SVG_COMMENT, SVG_LIKE } from "./assets";



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
        ${content ? content : '<div class="loader-wrapper"><img class="loader" src="https://i.gifer.com/ZKZg.gif" alt="Loading..."></div>'}
        </div>
        `;
    }

    async fetchData() {
        const url = this.getAttribute('src') || "/api/getfeed";

        try {
            const response = await fetch(url);
            const data = await response.json();

            // Проверяем: если это объект с полем posts, берем его.
            // Если это сам массив — используем его.
            // Если ничего из этого — создаем пустой массив.
            const posts = Array.isArray(data)
            ? data
            : (data.posts && Array.isArray(data.posts) ? data.posts : []);

            if (posts.length === 0) {
                this.render(`<span>Лента пуста или пришел неверный формат данных</span>`);
                return;
            }

            const feedHtml = posts.map((post: any) => `
            <div class="post-card">
                <div class="post-text">
                    <h2>${this.escapeHtml(post.title || 'Без названия')}</h2>

                    <p class="post-text-body">${this.escapeHtml(post.text || '')}</p>
                    <div class="post-info">
                        <img src="${SVG_COMMENT}" alt="Комменты">
                        <p>${this.escapeHtml(String(post.comment_count ?? '0'))}</p>
                        <img src="${SVG_LIKE}" alt="Лайки">
                        <p>${this.escapeHtml(String(post.like_count ?? '0'))}</p>
                    </div>
                </div>
                ${post.image_base64 ? `<img src="${LOGO}" alt="" class="post-img">` : ''}
            </div>
            `).join('');

            this.render(feedHtml);
        } catch (err) {
            this.render(`<span style="color: red;">Error: ${err}</span>`);
        }
    }

    // Вспомогательная функция для защиты от XSS (если текст придет от пользователя)
    private escapeHtml(unsafe: string) {
        return unsafe
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
    }
}
