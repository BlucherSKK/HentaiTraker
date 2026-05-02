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
    id:      number;
    title:   string | null;
    content: string;
    tags:    string | null;
    files:   string | null;
    time:    string;
}

// ----- ProfilePage -----

export class ProfilePage extends HTMLElement {
    private _ws?: HntWsConnection;
    user?: User;
    private _data: ProfileData | null = null;
    private _pendingAvatarFile: File | null = null;
    private _pendingAvatarPreview: string | null = null;

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

            const rawRoles = payload.roles;
            const rolesStr = Array.isArray(rawRoles)
            ? (rawRoles as string[]).join(', ')
            : (rawRoles as string | null) ?? null;

            this._data = {
                id:     payload.id     as number,
                name:   payload.name   as string,
                avatar: (payload.avatar ?? null) as string | null,
                      tags:   (payload.tags   ?? null) as string | null,
                      roles:  rolesStr,
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
        <div class="profile-name">${escHtml(d.name)}</div>
        ${d.roles ? `<div class="profile-roles">${escHtml(d.roles)}</div>` : ''}
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

            if (this._pendingAvatarPreview) URL.revokeObjectURL(this._pendingAvatarPreview);

            this._pendingAvatarFile    = file;
            this._pendingAvatarPreview = URL.createObjectURL(file);

            const img = this.querySelector('.profile-avatar') as HTMLImageElement;
            const ph  = this.querySelector('.profile-avatar-placeholder') as HTMLElement;
            if (img) { img.src = this._pendingAvatarPreview; img.style.display = ''; }
            if (ph)  { ph.style.display = 'none'; }
        });

        this.querySelector('#save-btn')?.addEventListener('click', () => this._save());
    }

    // ----- _save -----

    private async _save() {
        if (!this._ws) return;
        const status = this.querySelector('#profile-status') as HTMLElement;
        const btn    = this.querySelector('#save-btn')       as HTMLButtonElement;
        if (!status || !btn) return;

        btn.disabled = true;

        const checked = Array.from(this.querySelectorAll('.tag-cb:checked')) as HTMLInputElement[];
        const tags    = checked.map(el => el.value).join(',');

        const payload: Record<string, unknown> = { tags };

        if (this._pendingAvatarFile) {
            try {
                payload.avatar = await this._uploadAvatar(this._pendingAvatarFile);
            } catch (err: any) {
                status.textContent = `Ошибка загрузки аватара: ${err.message}`;
                btn.disabled = false;
                return;
            }
        }

        const cleanup: Array<() => void> = [];
        const done = () => { cleanup.forEach(f => f()); btn.disabled = false; };

        const offOk = this._ws.once('profile_updated', (_ev, p) => {
            done();
            if (!this.isConnected) return;
            if (this._data) {
                this._data.tags   = p.tags   as string | null;
                this._data.avatar = p.avatar as string | null;
            }
            if (this._pendingAvatarPreview) {
                URL.revokeObjectURL(this._pendingAvatarPreview);
                this._pendingAvatarPreview = null;
            }
            this._pendingAvatarFile = null;
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

    // ----- _uploadAvatar -----

    private async _uploadAvatar(file: File): Promise<string> {
        const token = await new Promise<string>((resolve, reject) => {
            const unsub = this._ws!.once('upload_token', (_ev, p) => {
                clearTimeout(timer);
                resolve(p.token as string);
            });
            const timer = setTimeout(() => { unsub(); reject(new Error('upload_token timeout')); }, 8_000);
            this._ws!.send('get_upload_token', {}).catch(err => { clearTimeout(timer); unsub(); reject(err); });
        });

        const fd = new FormData();
        fd.append('token', token);
        fd.append('file',  file);

        const res  = await fetch('/api/upload', { method: 'POST', body: fd });
        const json = await res.json() as { url?: string; error?: string };
        if (json.url) return json.url;
        throw new Error(json.error ?? 'upload_failed');
    }
}

// ----- helpers -----

function escHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
