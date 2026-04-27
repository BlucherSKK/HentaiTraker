// ----- post-card.ts -----

import { SVG_COMMENT, SVG_LIKE } from './assets';

// ----- types -----

export interface PostCardData {
    id?:           number;
    title?:        string | null;
    content:       string;
    tags?:         string | null;
    files?:        string | null;
    time?:         string;
    comment_count?: number;
    like_count?:    number;
}

export type PostCardMode = 'feed' | 'profile' | 'preview';

// ----- helpers -----

function escHtml(s: string): string {
    return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

const IMAGE_EXT_RE = /\.(jpg|jpeg|png|gif|webp)(\?.*)?$/i;

function isImageUrl(u: string): boolean {
    return u.startsWith('blob:') || IMAGE_EXT_RE.test(u);
}

function parseImages(files: string | null | undefined): string[] {
    if (!files) return [];
    try {
        const urls: string[] = JSON.parse(files);
        return urls.filter(isImageUrl).slice(0, 1);
    } catch { return []; }
}

function formatDate(iso: string): string {
    return new Date(iso).toLocaleDateString('ru-RU', {
        day: '2-digit', month: 'short', year: 'numeric',
    });
}

// ----- renderPostCard -----

export function renderPostCard(post: PostCardData, mode: PostCardMode): string {
    const title   = escHtml(post.title || 'Без названия');
    const content = escHtml(post.content || '');
    const images  = parseImages(post.files);
    const date    = post.time ? formatDate(post.time) : '';

    const tagsHtml = post.tags
    ? post.tags.split(',').map(t => `<span class="pc-card-tag">${escHtml(t.trim())}</span>`).join('')
    : '';

    // ----- только первая картинка -----
    const firstImage = images[0];
    const imagesHtml = firstImage
    ? `<div class="pc-card-images"><img src="${escHtml(firstImage)}" class="pc-card-img" loading="lazy"></div>`
    : '';

    const statsHtml = mode !== 'preview'
    ? `<div class="pc-card-stats">
    <img src="${SVG_COMMENT}" alt="comments" class="pc-card-stat-icon">
    <span>${post.comment_count ?? 0}</span>
    <img src="${SVG_LIKE}" alt="likes" class="pc-card-stat-icon">
    <span>${post.like_count ?? 0}</span>
    </div>`
    : `<div class="pc-card-stats"><span style="color:var(--ltextc);font-size:0.8em">0 💬 &nbsp; 0 ❤️</span></div>`;

    const clickable = mode !== 'preview' && post.id != null;

    return `
    <div class="pc-card post-card${clickable ? ' pc-card-clickable' : ''}"
    ${clickable ? `data-post-id="${post.id}"` : ''}>
    <div class="pc-card-body">
    <div class="pc-card-header">
    <h2 class="pc-card-title">${title}</h2>
    ${date ? `<span class="pc-card-date">${date}</span>` : ''}
    </div>
    ${tagsHtml ? `<div class="pc-card-tags">${tagsHtml}</div>` : ''}
    <p class="pc-card-content post-text-body">${content}</p>
    ${statsHtml}
    </div>
    ${imagesHtml}
    </div>`;
}

// ----- bindPostCardClicks -----

export function bindPostCardClicks(container: HTMLElement): void {
    container.addEventListener('click', e => {
        const card = (e.target as HTMLElement).closest<HTMLElement>('.pc-card-clickable');
        if (!card) return;
        const id = card.dataset.postId;
        if (id) {
            window.dispatchEvent(new CustomEvent('app-navigate', {
                detail: { page: 'post', postId: Number(id) }
            }));
        }
    });
}

// ----- POST_CARD_STYLES -----

export const POST_CARD_STYLES = `
.pc-card { display:flex; flex-direction:row; gap:10px; overflow:hidden; box-sizing:border-box; }
.pc-card-clickable { cursor:pointer; }
.pc-card-clickable:hover { background:var(--alt-bg); border-color:pink; }
.pc-card-body { flex:1; min-width:0; display:flex; flex-direction:column; gap:4px; }
.pc-card-header { display:flex; align-items:baseline; justify-content:space-between; gap:8px; }
.pc-card-title { margin:0; font-size:1rem; font-weight:600; color:var(--textc); white-space:nowrap; overflow:hidden; text-overflow:ellipsis; }
.pc-card-date { font-size:0.75em; color:var(--ltextc); white-space:nowrap; flex-shrink:0; }
.pc-card-tags { display:flex; flex-wrap:wrap; gap:4px; }
.pc-card-tag { font-size:0.72em; padding:2px 8px; background:var(--bgc); border:1px solid var(--border); border-radius:20px; color:var(--ltextc); }
.pc-card-content { margin:0; font-size:0.88em; color:var(--textc); line-height:1.5;
    display:-webkit-box; -webkit-line-clamp:3; -webkit-box-orient:vertical; overflow:hidden; }
    .pc-card-stats { display:flex; flex-direction:row; align-items:center; gap:6px; margin-top:4px; }
    .pc-card-stat-icon { width:16px; height:16px; filter:brightness(0) invert(1); }
    .pc-card-images { display:flex; flex-direction:column; justify-content:center; flex-shrink:0; }
    .pc-card-img { max-width:120px; max-height:120px; object-fit:cover; border-radius:4px; }
    `;
