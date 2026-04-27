import { SVG_COMMENT, SVG_LIKE } from "./assets";
import { bindPostCardClicks, POST_CARD_STYLES, renderPostCard } from "./post-card";



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
    connectedCallback() {
        this._injectStyles();
        this.render();
        this.fetchData();
    }

    render(content?: string) {
        this.innerHTML = `
        <div id="container" class="feed">
        ${content ?? '<div class="loader-wrapper"><div class="loader"></div></div>'}
        </div>`;
    }

    async fetchData() {
        const url = this.getAttribute('src') || '/api/getfeed';
        try {
            const response = await fetch(url);
            const data     = await response.json();
            const posts    = Array.isArray(data)
            ? data
            : (Array.isArray(data.posts) ? data.posts : []);

            if (!posts.length) {
                this.render('<span>Лента пуста или пришел неверный формат данных</span>');
                return;
            }

            const feedHtml = posts.map((post: any) => renderPostCard(post, 'feed')).join('');
            this.render(feedHtml);

            const container = this.querySelector<HTMLElement>('#container');
            if (container) bindPostCardClicks(container);
        } catch (e) {
            this.render('<span>Ошибка загрузки ленты</span>');
        }
    }

    private _injectStyles() {
        if (document.getElementById('post-card-styles')) return;
        const s = document.createElement('style');
        s.id = 'post-card-styles';
        s.textContent = POST_CARD_STYLES;
        document.head.appendChild(s);
    }
}
