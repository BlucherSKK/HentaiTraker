// ----- post-card.ts -----

import { SVG_COMMENT, SVG_LIKE } from './assets';

// ----- types -----

export interface PostCardData {
    id?:            number;
    title?:         string | null;
    content:        string;
    tags?:          string | null;
    files?:         string | null;
    time?:          string;
    comment_count?: number;
    like_count?:    number;
}

export type PostCardMode = 'feed' | 'profile' | 'preview' | 'sidebar';

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

    const firstImage = images[0];
    const imageHtml  = firstImage
    ? `<div class="pc-card-image"><img src="${escHtml(firstImage)}" class="pc-card-img" loading="lazy"></div>`
    : '';

    let statsHtml = '';
    if (mode !== 'preview' && mode !== 'sidebar') {
        statsHtml = `<div class="pc-card-stats">
        <img src="${SVG_COMMENT}" alt="comments" class="pc-card-stat-icon">
        <span>${post.comment_count ?? 0}</span>
        <img src="${SVG_LIKE}" alt="likes" class="pc-card-stat-icon">
        <span>${post.like_count ?? 0}</span>
        </div>`;
    } else if (mode === 'preview') {
        statsHtml = `<div class="pc-card-stats"><span style="color:var(--ltextc);font-size:0.8em">0 💬 &nbsp; 0 ❤️</span></div>`;
    }

    const clickable = mode !== 'preview' && post.id != null;

    return `
    <div class="pc-card post-card${clickable ? ' pc-card-clickable' : ''}"
    ${clickable ? `data-post-id="${post.id}" ` : ''}>
    ${imageHtml}
    <div class="pc-card-body">
    <div class="pc-card-header">
    <h2 class="pc-card-title">${title}</h2>
    ${date ? `<span class="pc-card-date">${date}</span>` : ''}
    </div>
    ${tagsHtml && mode !== 'sidebar' ? `<div class="pc-card-tags">${tagsHtml}</div>` : ''}
    <p class="pc-card-content post-text-body">${content}</p>
    ${statsHtml}
    </div>
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
                detail: { page: 'post-page', postId: Number(id) }
            }));
        }
    });
}

// ----- POST_CARD_STYLES -----

export const POST_CARD_STYLES = `
.pc-card {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-sizing: border-box;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--alt-bg);
}
.pc-card-clickable { cursor: pointer; }
.pc-card-clickable:hover { background: var(--alt-bg); border-color: pink; }

.pc-card-image {
    width: 100%;
    flex-shrink: 0;
    overflow: hidden;
    background: #000;
}
.pc-card-img {
    width: 100%;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    display: block;
}

.pc-card-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
}

.pc-card-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
}
.pc-card-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--textc);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.pc-card-date {
    font-size: 0.72em;
    color: var(--ltextc);
    white-space: nowrap;
    flex-shrink: 0;
}

.pc-card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
}
.pc-card-tag {
    font-size: 0.72em;
    padding: 2px 8px;
    background: var(--bgc);
    border: 1px solid var(--border);
    border-radius: 20px;
    color: var(--ltextc);
}

.pc-card-content {
    margin: 0;
    font-size: 0.88em;
    color: var(--ltextc);
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
}

.pc-card-stats {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
}
.pc-card-stat-icon {
    width: 15px;
    height: 15px;
    filter: brightness(0) invert(1);
}
`;
