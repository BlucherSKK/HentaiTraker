// ----- post-page.ts -----

import { PostCardData } from './post-card';
import { HntWsConnection } from './ws';

interface FullPost extends PostCardData {
    author_name?: string;
}

function escHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString('ru-RU', {
        day: '2-digit', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit',
    });
}

const IMAGE_EXT_RE = /\.(jpg|jpeg|png|gif|webp)(\?.*)?$/i;

function parseImages(files: string | null | undefined): string[] {
    if (!files) return [];
    try {
        const urls: string[] = JSON.parse(files);
        return urls.filter(u => u.startsWith('blob:') || IMAGE_EXT_RE.test(u));
    } catch { return []; }
}

// ----- PostPage -----

export class PostPage extends HTMLElement {
    private _postId: number | null = null;

    get postId(): number | null { return this._postId; }
    set postId(val: number | null) {
        this._postId = val;
        if (this.isConnected) this._load();
    }

    connectedCallback() {
        this._injectStyles();
        this._renderLoading();
        if (this._postId != null) this._load();
    }

    private async _load() {
        if (this._postId == null) return;
        this._renderLoading();
        try {
            const res = await fetch(`/api/post/${this._postId}`);
            if (!res.ok) { this._renderError('Пост не найден'); return; }
            const post: FullPost = await res.json();
            this._renderPost(post);
        } catch {
            this._renderError('Ошибка загрузки');
        }
    }

    private _renderLoading() {
        this.innerHTML = `<div class="pp-wrap"><div class="loader-wrapper"><div class="loader"></div></div></div>`;
    }

    private _renderError(msg: string) {
        this.innerHTML = `<div class="pp-wrap"><p class="pp-error">${escHtml(msg)}</p></div>`;
    }

    private _renderPost(post: FullPost) {
        const title    = escHtml(post.title || 'Без названия');
        const date     = post.time ? formatDate(post.time) : '';
        const images   = parseImages(post.files);

        const tagsHtml = post.tags
        ? post.tags.split(',').map(t => `<span class="pp-tag">${escHtml(t.trim())}</span>`).join('')
        : '';

        const imagesHtml = images.map(src =>
        `<img src="${escHtml(src)}" class="pp-img" loading="lazy">`
        ).join('');

        this.innerHTML = `
        <div class="pp-wrap">
        <div class="pp-header">
        <button class="pp-back" id="pp-back">← Назад</button>
        </div>
        <article class="pp-article">
        <h1 class="pp-title">${title}</h1>
        <div class="pp-meta">
        ${date ? `<span class="pp-date">${date}</span>` : ''}
        ${post.author_name ? `<span class="pp-author">${escHtml(post.author_name)}</span>` : ''}
        </div>
        ${tagsHtml ? `<div class="pp-tags">${tagsHtml}</div>` : ''}
        ${imagesHtml ? `<div class="pp-images">${imagesHtml}</div>` : ''}
        <div class="pp-content post-text-body">${escHtml(post.content)}</div>
        </article>
        </div>`;

        this.querySelector('#pp-back')?.addEventListener('click', () => {
            window.dispatchEvent(new CustomEvent('app-navigate', { detail: { page: 'feeds' } }));
            history.back();
        });
    }

    private _injectStyles() {
        if (document.getElementById('post-page-styles')) return;
        const s = document.createElement('style');
        s.id = 'post-page-styles';
        s.textContent = POST_PAGE_STYLES;
        document.head.appendChild(s);
    }
}

// ----- styles -----

export const POST_PAGE_STYLES = `
.pp-wrap { max-width: 720px; margin: 0 auto; padding: 1.5em 1em; }
.pp-header { margin-bottom: 1em; }
.pp-back { background: none; border: 1px solid var(--border); color: var(--ltextc); border-radius: 6px; padding: 4px 12px; cursor: pointer; font-size: 0.85em; }
.pp-back:hover { color: var(--textc); border-color: var(--textc); }
.pp-article { display: flex; flex-direction: column; gap: 0.75em; }
.pp-title { margin: 0; font-size: 1.5rem; color: var(--textc); }
.pp-meta { display: flex; gap: 1em; font-size: 0.8em; color: var(--ltextc); }
.pp-tags { display: flex; flex-wrap: wrap; gap: 4px; }
.pp-tag { font-size: 0.72em; padding: 2px 8px; background: var(--bgc); border: 1px solid var(--border); border-radius: 20px; color: var(--ltextc); }
.pp-images { display: flex; flex-direction: column; gap: 0.5em; }
.pp-img { max-width: 100%; border-radius: 8px; object-fit: contain; }
.pp-content { font-size: 0.95em; line-height: 1.7; color: var(--textc); white-space: pre-wrap; }
.pp-error { color: var(--ltextc); text-align: center; padding: 2em 0; }
`;
