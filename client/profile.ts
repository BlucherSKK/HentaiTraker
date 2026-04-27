import { User } from "./app";
import { bindPostCardClicks, PostCardData, renderPostCard } from "./post-card";
import { HntWsConnection } from "./ws";

const TAG_LABELS: Record<string, string> = {
    hnt: 'Хентай',
    any: 'Любой',
};

const AVAILABLE_TAGS = Object.keys(TAG_LABELS);

interface ProfileData {
    id:     number;
    name:   string;
    avatar: string | null;
    tags:   string | null;
    roles:  string | null;
}

interface PostItem {
    id:        number;
    title:     string | null;
    content:   string;
    tags:      string | null;
    files:     string | null;
    time:      string;
}

// ----- ProfilePage -----

export class ProfilePage extends HTMLElement {
    private _ws?: HntWsConnection;
    user?: User;
    private _data: ProfileData | null = null;
    private _pendingAvatar: string | null = null;

    get ws(): HntWsConnection | undefined { return this._ws; }
    set ws(val: HntWsConnection | undefined) {
        this._ws = val;
        if (val && this.isConnected) this._loadProfile();
    }

    connectedCallback() {
        this.render();
        if (this._ws) this._loadProfile();
    }

    private render() {
        this.innerHTML = `<div class="profile-page"><p class="profile-loading">Загрузка профиля...</p></div>`;
    }

    private _loadProfile() {
        if (!this._ws) return;

        this._ws.once('profile_ok', (_ev, payload) => {
            if (!this.isConnected) return;
            this._data = {
                id:     payload.id     as number,
                name:   payload.name   as string,
                avatar: (payload.avatar ?? null) as string | null,
                      tags:   (payload.tags  ?? null) as string | null,
                      roles:  (payload.roles ?? null) as string | null,
            };
            this._renderProfile();
            this._loadPosts();
        });

        this._ws.send('profile_get', {}).catch(console.error);
    }

    private _loadPosts() {
        if (!this._ws || !this._data) return;

        this._ws.once('user_posts', (_ev, payload) => {
            if (!this.isConnected) return;
            const posts = (payload.posts ?? []) as PostItem[];
            this._renderPosts(posts);
        });

        this._ws.send('user_posts', { limit: 50 }).catch(console.error);
    }

    private _renderProfile() {
        if (!this._data) return;
        const d = this._data;
        const currentTags = d.tags
        ? d.tags.split(',').map(t => t.trim()).filter(Boolean)
        : [];

        const tagsHtml = AVAILABLE_TAGS.map(tag => `
        <label class="profile-tag-label">
        <input type="checkbox" class="tag-cb" value="${tag}" ${currentTags.includes(tag) ? 'checked' : ''}>
        <span>${TAG_LABELS[tag]}</span>
        </label>`).join('');

        const wrap = this.querySelector('.profile-page')!;
        wrap.innerHTML = `
        <div class="profile-card">
        <div class="profile-header-section">
        <div class="profile-avatar-wrap">
        <img class="profile-avatar" src="${d.avatar ?? ''}" alt="avatar"
        style="${d.avatar ? '' : 'display:none'}">
        <div class="profile-avatar-placeholder"
        style="${d.avatar ? 'display:none' : ''}">?</div>
        <input type="file" id="avatar-file" accept="image/*" hidden>
        <button type="button" class="profile-avatar-btn" id="avatar-btn">Сменить аватар</button>
        </div>
        <div class="profile-meta">
        <div class="profile-id">ID: ${d.id}</div>
        <div class="profile-name">${d.name}</div>
        ${d.roles ? `<div class="profile-roles">${d.roles}</div>` : ''}
        </div>
        </div>
        <div class="profile-section">
        <h3 class="profile-section-title">Теги</h3>
        <div class="profile-tags-grid">${tagsHtml}</div>
        </div>
        <div class="profile-actions">
        <button class="profile-save-btn" id="save-btn">Сохранить</button>
        <span class="profile-status" id="profile-status"></span>
        </div>
        </div>
        <div class="profile-posts-section" id="profile-posts-section">
        <div class="profile-posts-header">Мои посты</div>
        <div class="profile-posts-list" id="profile-posts-list">
        <span class="profile-posts-loading">Загрузка постов...</span>
        </div>
        </div>`;

        this._bindProfileEvents();
    }

    private _renderPosts(posts: PostItem[]) {
        const list = this.querySelector<HTMLElement>('#profile-posts-list');
        if (!list) return;

        if (!posts.length) {
            list.innerHTML = `<span class="profile-posts-empty">Постов пока нет</span>`;
            return;
        }

        list.innerHTML = posts.map(post => renderPostCard(post as PostCardData, 'profile')).join('');
        bindPostCardClicks(list);
    }

    private _bindProfileEvents() {
        this.querySelector('#avatar-btn')?.addEventListener('click', () => {
            (this.querySelector('#avatar-file') as HTMLInputElement)?.click();
        });

        this.querySelector('#avatar-file')?.addEventListener('change', e => {
            const file = (e.target as HTMLInputElement).files?.[0];
            if (!file) return;
            const reader = new FileReader();
            reader.onload = () => {
                this._pendingAvatar = reader.result as string;
                const img = this.querySelector('.profile-avatar') as HTMLImageElement;
                const ph  = this.querySelector('.profile-avatar-placeholder') as HTMLElement;
                if (img) { img.src = this._pendingAvatar; img.style.display = ''; }
                if (ph)  { ph.style.display = 'none'; }
            };
            reader.readAsDataURL(file);
        });

        this.querySelector('#save-btn')?.addEventListener('click', () => this._save());
    }

    private _save() {
        if (!this._ws) return;
        const status = this.querySelector('#profile-status') as HTMLElement;
        const btn    = this.querySelector('#save-btn')       as HTMLButtonElement;
        if (!status || !btn) return;

        btn.disabled = true;

        const checked = Array.from(this.querySelectorAll('.tag-cb:checked')) as HTMLInputElement[];
        const tags    = checked.map(el => el.value).join(',');

        const payload: Record<string, unknown> = { tags };
        if (this._pendingAvatar) payload.avatar = this._pendingAvatar;

        const cleanup: Array<() => void> = [];
        const done = () => { cleanup.forEach(f => f()); btn.disabled = false; };

        const offOk = this._ws.once('profile_updated', (_ev, p) => {
            done();
            if (!this.isConnected) return;
            if (this._data) {
                this._data.tags   = p.tags   as string | null;
                this._data.avatar = p.avatar as string | null;
            }
            this._pendingAvatar = null;
            status.textContent = 'Сохранено!';
            setTimeout(() => { if (this.isConnected) status.textContent = ''; }, 2500);
        });

        const offErr = this._ws.once('error', (_ev, p) => {
            done();
            if (!this.isConnected) return;
            status.textContent = `Ошибка: ${p.code}`;
        });

        cleanup.push(offOk, offErr);

        this._ws.send('profile_update', payload).catch(err => {
            done();
            if (this.isConnected) status.textContent = `Ошибка: ${err}`;
        });
    }
}

function escHtml(s: string): string {
    return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}
