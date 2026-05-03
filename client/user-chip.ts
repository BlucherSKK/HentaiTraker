import { getProfileCache, ProfileCache } from './store';

// ----- UserChip -----

export interface UserChipData {
    name:   string;
    avatar: string | null;
    score:  number;
}

const STYLES = `
.user-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.5em;
    padding: 0.25em 0.6em 0.25em 0.25em;
    border-radius: 2em;
    border: 1px solid var(--border);
    background: var(--alt-bg);
    cursor: default;
    user-select: none;
    max-width: 220px;
    box-sizing: border-box;
}
.user-chip__avatar {
    width: 1.8em;
    height: 1.8em;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--bgc);
    border: 1px solid var(--border);
}
.user-chip__placeholder {
    width: 1.8em;
    height: 1.8em;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--bgc);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75em;
    color: var(--ltextc);
}
.user-chip__name {
    font-size: 0.88em;
    font-weight: 600;
    color: var(--textc);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
.user-chip__score {
    font-size: 0.78em;
    color: var(--accentc);
    white-space: nowrap;
    flex-shrink: 0;
}
`;

function injectStyles(): void {
    if (document.getElementById('user-chip-styles')) return;
    const s = document.createElement('style');
    s.id          = 'user-chip-styles';
    s.textContent = STYLES;
    document.head.appendChild(s);
}

function renderChipHtml(d: UserChipData): string {
    const avatarHtml = d.avatar
    ? `<img class="user-chip__avatar" src="${d.avatar}" alt="">`
    : `<div class="user-chip__placeholder">?</div>`;

    return `
    ${avatarHtml}
    <span class="user-chip__name">${escHtml(d.name)}</span>
    <span class="user-chip__score">⭐ ${d.score}</span>
    `;
}

function escHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ----- custom element -----
// Атрибуты:
//   data-self  — отображает текущего юзера из store (обновляется через refresh())
//   data-name, data-avatar, data-score — явные данные (для постов, чатов)

export class UserChip extends HTMLElement {
    private _data: UserChipData | null = null;

    connectedCallback() {
        injectStyles();
        this.className = 'user-chip';

        if (this.hasAttribute('data-self')) {
            this._renderFromCache();
        } else {
            this._renderFromAttrs();
        }
    }

    static get observedAttributes() {
        return ['data-name', 'data-avatar', 'data-score'];
    }

    attributeChangedCallback() {
        if (this.isConnected && !this.hasAttribute('data-self')) {
            this._renderFromAttrs();
        }
    }

    refresh(): void {
        if (this.hasAttribute('data-self')) this._renderFromCache();
    }

    setData(d: UserChipData): void {
        this._data = d;
        this._render(d);
    }

    private _renderFromCache(): void {
        const cache = getProfileCache();
        if (cache) {
            this._render(cache);
        } else {
            this._renderFromAttrs();
        }
    }

    private _renderFromAttrs(): void {
        this._render({
            name:   this.getAttribute('data-name')   ?? '?',
                     avatar: this.getAttribute('data-avatar'),
                     score:  Number(this.getAttribute('data-score') ?? 0),
        });
    }

    private _render(d: UserChipData): void {
        this._data        = d;
        this.innerHTML    = renderChipHtml(d);
    }
}
