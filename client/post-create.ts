// post-create.ts

import { HntWsConnection } from "./ws";

// ----- markdown renderer -----

function renderMarkdown(raw: string): string {
    const lines = raw.split('\n');
    const out: string[] = [];
    let inList = false;
    let inChecklist = false;

    for (const line of lines) {
        const h2 = line.match(/^##\s+(.+)/);
        if (h2) {
            if (inList)      { out.push('</ul>'); inList = false; }
            if (inChecklist) { out.push('</ul>'); inChecklist = false; }
            out.push(`<h2>${escMd(h2[1])}</h2>`);
            continue;
        }
        const chk = line.match(/^- \[( |x)\] (.+)/i);
        if (chk) {
            if (inList) { out.push('</ul>'); inList = false; }
            if (!inChecklist) { out.push('<ul class="pc-checklist">'); inChecklist = true; }
            const checked = chk[1].toLowerCase() === 'x';
            out.push(`<li><input type="checkbox" ${checked ? 'checked' : ''} disabled> ${inlineRender(chk[2])}</li>`);
            continue;
        }
        const li = line.match(/^- (.+)/);
        if (li) {
            if (inChecklist) { out.push('</ul>'); inChecklist = false; }
            if (!inList) { out.push('<ul class="pc-list">'); inList = true; }
            out.push(`<li>${inlineRender(li[1])}</li>`);
            continue;
        }
        if (inList)      { out.push('</ul>'); inList = false; }
        if (inChecklist) { out.push('</ul>'); inChecklist = false; }
        if (line.trim() === '') { out.push('<br>'); } else { out.push(`<p>${inlineRender(line)}</p>`); }
    }
    if (inList)      out.push('</ul>');
    if (inChecklist) out.push('</ul>');
    return out.join('');
}

function inlineRender(text: string): string {
    return text
    .replace(/!\[([^\]]*)\]\(([^)]+)\)/g, (_, alt, src) => {
        // ----- FIX: blob и абсолютные URL не оборачиваем в /api/files/ -----
        const url = (src.startsWith('/') || src.startsWith('blob:') || src.startsWith('http'))
        ? src
        : `/api/files/${escAttr(src)}`;
        return `<img src="${url}" alt="${escAttr(alt)}" class="pc-inline-img">`;
    })
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, label, href) =>
    `<a href="${escAttr(href)}" target="_blank" rel="noopener">${escMd(label)}</a>`
    );
}

function escMd(s: string): string   { return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }
function escAttr(s: string): string { return s.replace(/"/g,'&quot;').replace(/</g,'&lt;'); }
function escapeRegex(s: string): string { return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

// ----- tag validation -----

const TAG_RE = /^[a-z_]+$/;
const SUGGESTED_TAGS = ['hentai', 'art', 'discussion', 'video', 'music', 'games', 'news', 'fan_art', 'fan_fiction', 'review'];

function isValidTag(t: string): boolean { return TAG_RE.test(t); }

// ----- types -----

interface PendingImage { file: File; localUrl: string; placeholder: string; }

let _imgCounter = 0;

// ----- PostCreatePage -----

export class PostCreatePage extends HTMLElement {
    public ws: HntWsConnection | undefined;

    private _images: PendingImage[]    = [];
    private _selectedTags: Set<string> = new Set(['any']);
    private _busy = false;

    connectedCallback() { this._attachStyles(); this.render(); }

    private render() {
        this.innerHTML = `
        <div class="pc-wrap">
        <div class="pc-header">
        <span class="pc-title">Новый пост</span>
        <button class="pc-cancel-btn" id="pc-cancel">Отмена</button>
        </div>

        <input class="pc-input-title" id="pc-post-title" type="text"
        placeholder="Заголовок (необязательно)" maxlength="200">

        <div class="pc-tags-section">
        <span class="pc-tags-label">Теги <span class="pc-tags-hint">только a–z и _</span></span>
        <div class="pc-tags-input-row">
        <input class="pc-tag-input" id="pc-tag-input" type="text"
        placeholder="введи тег и нажми Enter или ," maxlength="40" autocomplete="off">
        <div class="pc-tag-suggestions" id="pc-tag-suggestions"></div>
        </div>
        <div class="pc-tags-selected" id="pc-tags-selected"></div>
        <div class="pc-tag-error" id="pc-tag-error"></div>
        </div>

        <div class="pc-editor-area">
        <div class="pc-drop-zone" id="pc-drop-zone">
        <span class="pc-drop-hint">Перетащите картинку — плейсхолдер вставится в текст и заменится серверным именем при публикации</span>
        </div>
        <div class="pc-panes">
        <textarea class="pc-textarea" id="pc-textarea"
        placeholder="## Заголовок&#10;- список&#10;- [ ] чеклист&#10;![alt](__img_0__)"></textarea>
        <div class="pc-preview" id="pc-preview"></div>
        </div>
        </div>

        <div class="pc-feed-preview-label">Превью в ленте</div>
        <div class="pc-feed-preview post-card" id="pc-feed-preview">
        <div class="post-text">
        <h2 id="pcfp-title" class="pcfp-title-placeholder">Без названия</h2>
        <p class="post-text-body" id="pcfp-body"></p>
        <div class="post-info">
        <span style="color:var(--ltextc);font-size:0.8em">0 💬 &nbsp; 0 ❤️</span>
        </div>
        </div>
        <div id="pcfp-img-wrap"></div>
        </div>

        <div class="pc-thumbs" id="pc-thumbs"></div>

        <div class="pc-footer">
        <div class="pc-progress-wrap" id="pc-progress-wrap" style="display:none">
        <div class="pc-progress-bar" id="pc-progress-bar"></div>
        <span class="pc-progress-label" id="pc-progress-label"></span>
        </div>
        <div class="pc-footer-row">
        <span class="pc-status" id="pc-status"></span>
        <button class="pc-submit-btn" id="pc-submit">Опубликовать</button>
        </div>
        </div>
        </div>`;

        this._renderSelectedTags();
        this._bindEvents();
    }

    private _bindEvents() {
        this.querySelector('#pc-cancel')?.addEventListener('click', () => this._cancel());
        this.querySelector('#pc-submit')?.addEventListener('click', () => this._submit());

        const ta = this.querySelector<HTMLTextAreaElement>('#pc-textarea');
        ta?.addEventListener('input', () => this._updatePreview());

        const titleInput = this.querySelector<HTMLInputElement>('#pc-post-title');
        titleInput?.addEventListener('input', () => this._updateFeedPreview());

        const dz = this.querySelector<HTMLElement>('#pc-drop-zone');
        if (dz) {
            dz.addEventListener('dragover',  e => { e.preventDefault(); dz.classList.add('pc-drop-over'); });
            dz.addEventListener('dragleave', () => dz.classList.remove('pc-drop-over'));
            dz.addEventListener('drop', e => {
                e.preventDefault();
                dz.classList.remove('pc-drop-over');
                const file = e.dataTransfer?.files[0];
                if (file) this._addImage(file);
            });
        }

        this._bindTagInput();
    }

    // ----- tags input -----

    private _bindTagInput() {
        const input = this.querySelector<HTMLInputElement>('#pc-tag-input');
        if (!input) return;

        input.addEventListener('input', () => {
            this._clearTagError();
            this._renderSuggestions(input.value.trim().toLowerCase());
        });

        input.addEventListener('keydown', e => {
            if (e.key === 'Enter' || e.key === ',') {
                e.preventDefault();
                this._commitTag(input.value);
                input.value = '';
                this._renderSuggestions('');
            } else if (e.key === 'Backspace' && input.value === '') {
                const last = [...this._selectedTags].filter(t => t !== 'any').pop();
                if (last) { this._selectedTags.delete(last); this._renderSelectedTags(); }
            }
        });

        input.addEventListener('blur', () => {
            setTimeout(() => this._renderSuggestions(''), 150);
        });
    }

    private _commitTag(raw: string) {
        const tag = raw.trim().toLowerCase().replace(/,/g, '');
        if (!tag) return;
        if (!isValidTag(tag)) { this._setTagError(`«${tag}» — недопустимые символы.`); return; }
        if (this._selectedTags.size >= 5) { this._setTagError('Максимум 5 тегов'); return; }
        this._selectedTags.delete('any');
        this._selectedTags.add(tag);
        this._renderSelectedTags();
    }

    private _renderSuggestions(query: string) {
        const box = this.querySelector<HTMLElement>('#pc-tag-suggestions');
        if (!box) return;
        const matches = query ? SUGGESTED_TAGS.filter(t => t.startsWith(query) && !this._selectedTags.has(t)) : [];
        if (!matches.length) { box.innerHTML = ''; box.style.display = 'none'; return; }
        box.style.display = 'block';
        box.innerHTML = matches.map(t => `<div class="pc-tag-sug-item" data-tag="${t}">${t}</div>`).join('');
        box.querySelectorAll<HTMLElement>('.pc-tag-sug-item').forEach(item => {
            item.addEventListener('mousedown', e => {
                e.preventDefault();
                this._commitTag(item.dataset.tag ?? '');
                const input = this.querySelector<HTMLInputElement>('#pc-tag-input');
                if (input) { input.value = ''; }
                this._renderSuggestions('');
            });
        });
    }

    private _renderSelectedTags() {
        const sel = this.querySelector<HTMLElement>('#pc-tags-selected');
        if (!sel) return;
        sel.innerHTML = '';
        for (const tag of this._selectedTags) {
            const chip = document.createElement('div');
            chip.className = `pc-tag-chip${tag === 'any' ? ' pc-tag-any' : ''}`;
            chip.textContent = tag;
            if (tag !== 'any') {
                const rm = document.createElement('button');
                rm.className   = 'pc-tag-rm';
                rm.textContent = '✕';
                rm.addEventListener('click', () => {
                    this._selectedTags.delete(tag);
                    this._renderSelectedTags();
                });
                chip.appendChild(rm);
            }
            sel.appendChild(chip);
        }
    }

    private _setTagError(msg: string) {
        const el = this.querySelector<HTMLElement>('#pc-tag-error');
        if (el) el.textContent = msg;
    }

    private _clearTagError() {
        const el = this.querySelector<HTMLElement>('#pc-tag-error');
        if (el) el.textContent = '';
    }

    // ----- preview -----

    private _resolveText(): string {
        const ta = this.querySelector<HTMLTextAreaElement>('#pc-textarea');
        let text = ta?.value ?? '';
        for (const img of this._images) {
            text = text.replace(new RegExp(escapeRegex(img.placeholder), 'g'), img.localUrl);
        }
        return text;
    }

    private _updatePreview() {
        const preview = this.querySelector<HTMLElement>('#pc-preview');
        if (preview) preview.innerHTML = renderMarkdown(this._resolveText());
        this._updateFeedPreview();
    }

    // ----- feed card preview -----

    private _updateFeedPreview() {
        const title   = (this.querySelector<HTMLInputElement>('#pc-post-title')?.value ?? '').trim();
        const ta      = this.querySelector<HTMLTextAreaElement>('#pc-textarea');
        const rawText = ta?.value ?? '';

        const titleEl   = this.querySelector<HTMLElement>('#pcfp-title');
        const bodyEl    = this.querySelector<HTMLElement>('#pcfp-body');
        const imgWrap   = this.querySelector<HTMLElement>('#pcfp-img-wrap');

        if (titleEl) {
            titleEl.textContent = title || 'Без названия';
            titleEl.classList.toggle('pcfp-title-placeholder', !title);
        }

        if (bodyEl) {
            const plain = rawText.replace(/!\[[^\]]*\]\([^)]+\)/g, '').replace(/#+\s*/g, '').trim();
            bodyEl.textContent = plain;
        }

        if (imgWrap) {
            const firstImg = this._images[0];
            if (firstImg) {
                imgWrap.innerHTML = `<img src="${firstImg.localUrl}" alt="" class="post-img">`;
            } else {
                imgWrap.innerHTML = '';
            }
        }
    }

    // ----- images -----

    private _addImage(file: File) {
        const allowed = ['image/jpeg','image/png','image/gif','image/webp'];
        if (!allowed.includes(file.type)) { this._setStatus('Только jpg/png/gif/webp'); return; }

        const placeholder = `__img_${_imgCounter++}__`;
        const localUrl    = URL.createObjectURL(file);
        this._images.push({ file, localUrl, placeholder });

        const ta = this.querySelector<HTMLTextAreaElement>('#pc-textarea');
        if (ta) {
            const insert = `![](${placeholder})`;
            const start  = ta.selectionStart ?? ta.value.length;
            const end    = ta.selectionEnd   ?? ta.value.length;
            ta.value = ta.value.slice(0, start) + insert + ta.value.slice(end);
            ta.selectionStart = ta.selectionEnd = start + insert.length;
            this._updatePreview();
        }

        this._renderThumb({ file, localUrl, placeholder });
        this._setStatus(`Добавлено: ${file.name} → ${placeholder}`);
    }

    private _renderThumb(img: PendingImage) {
        const thumbs = this.querySelector<HTMLElement>('#pc-thumbs');
        if (!thumbs) return;
        const wrap = document.createElement('div');
        wrap.className = 'pc-thumb-wrap';
        wrap.innerHTML = `
        <img src="${img.localUrl}" class="pc-thumb-img" alt="${img.placeholder}">
        <span class="pc-thumb-name">${img.placeholder}</span>
        <button class="pc-thumb-rm" title="Удалить">✕</button>`;
        wrap.querySelector('.pc-thumb-rm')?.addEventListener('click', () => {
            this._images = this._images.filter(i => i.placeholder !== img.placeholder);
            URL.revokeObjectURL(img.localUrl);
            wrap.remove();
            this._updatePreview();
        });
        thumbs.appendChild(wrap);
    }

    // ----- submit -----

    private async _submit() {
        if (this._busy) return;
        if (!this.ws) { this._setStatus('WS недоступен'); return; }

        const title   = (this.querySelector<HTMLInputElement>('#pc-post-title')?.value ?? '').trim();
        const ta      = this.querySelector<HTMLTextAreaElement>('#pc-textarea');
        const content = (ta?.value ?? '').trim();
        if (!content) { this._setStatus('Текст поста не может быть пустым'); return; }

        const pendingInput = (this.querySelector<HTMLInputElement>('#pc-tag-input')?.value ?? '').trim();
        if (pendingInput) this._commitTag(pendingInput);

        this._busy = true;
        this._setSubmitEnabled(false);

        const uploadedUrls: string[] = [];
        const total = this._images.length;
        let   workingContent = content;

        try {
            for (let i = 0; i < total; i++) {
                const img = this._images[i];
                this._setProgress((i / (total + 1)) * 90, `Загрузка ${i + 1} из ${total}: ${img.file.name}`);
                const serverFilename = await this._uploadOne(img.file);
                uploadedUrls.push(`/api/files/${serverFilename}`);
                workingContent = workingContent.replace(
                    new RegExp(escapeRegex(img.placeholder), 'g'),
                                                        serverFilename
                );
            }

            this._setProgress(95, 'Публикуем пост…');

            const files = uploadedUrls.length > 0 ? JSON.stringify(uploadedUrls) : undefined;

            await new Promise<void>((resolve, reject) => {
                // ----- once() возвращает () => void (unsub), а не this -----
                const unsub = this.ws!.once('post_created', () => {
                    clearTimeout(timer);
                    resolve();
                });
                const timer = setTimeout(() => {
                    unsub();
                    reject(new Error('timeout'));
                }, 15_000);

                this.ws!.send('create_post', {
                    title,
                    content: workingContent,
                    files,
                    tags: [...this._selectedTags].join(','),
                }).catch(err => { clearTimeout(timer); unsub(); reject(err); });
            });

            this._setProgress(100, 'Готово!');
            setTimeout(() => this._navigate('feeds'), 800);
        } catch (err: any) {
            this._setStatus(`Ошибка: ${err.message}`);
            this._busy = false;
            this._setSubmitEnabled(true);
            this._hideProgress();
        }
    }

    private async _uploadOne(file: File): Promise<string> {
        const fd = new FormData();
        fd.append('file', file);
        const res  = await fetch('/api/upload', { method: 'POST', body: fd });
        const json = await res.json() as { filename?: string; error?: string };
        if (json.filename) return json.filename;
        throw new Error(json.error ?? 'upload failed');
    }

    private async _cancel() {
        this._images.forEach(i => URL.revokeObjectURL(i.localUrl));
        this._images = [];
        this._navigate('feeds');
    }

    private _setStatus(msg: string) {
        const el = this.querySelector<HTMLElement>('#pc-status');
        if (el) el.textContent = msg;
    }

    private _setSubmitEnabled(on: boolean) {
        const btn = this.querySelector<HTMLButtonElement>('#pc-submit');
        if (btn) btn.disabled = !on;
    }

    private _setProgress(pct: number, label: string) {
        const wrap = this.querySelector<HTMLElement>('#pc-progress-wrap');
        const bar  = this.querySelector<HTMLElement>('#pc-progress-bar');
        const lbl  = this.querySelector<HTMLElement>('#pc-progress-label');
        if (wrap) wrap.style.display = '';
        if (bar)  bar.style.width = `${Math.min(100, pct)}%`;
        if (lbl)  lbl.textContent = label;
    }

    private _hideProgress() {
        const wrap = this.querySelector<HTMLElement>('#pc-progress-wrap');
        if (wrap) wrap.style.display = 'none';
    }

    private _navigate(page: string) {
        window.dispatchEvent(new CustomEvent('app-navigate', { detail: { page } }));
    }

    private _attachStyles() {
        if (document.getElementById('post-create-styles')) return;
        const s = document.createElement('style');
        s.id = 'post-create-styles';
        s.textContent = POST_CREATE_STYLES;
        document.head.appendChild(s);
    }
}

const POST_CREATE_STYLES = `
.pc-wrap { display:flex; flex-direction:column; gap:12px; padding:20px; max-width:1100px; margin:0 auto; color:var(--textc); }
.pc-header { display:flex; align-items:center; justify-content:space-between; }
.pc-title { font-size:1.3rem; font-weight:bold; }
.pc-cancel-btn { padding:6px 16px; cursor:pointer; background:transparent; border:1px solid var(--border); border-radius:4px; color:var(--textc); }
.pc-cancel-btn:hover { background:var(--alt-bg); }
.pc-input-title { width:100%; padding:10px; font-size:1rem; border:1px solid var(--border); border-radius:4px; background:var(--bgc); color:var(--textc); box-sizing:border-box; }

.pc-tags-section { display:flex; flex-direction:column; gap:6px; padding:12px; background:var(--alt-bg); border:1px solid var(--border); border-radius:6px; }
.pc-tags-label { font-size:0.9rem; font-weight:600; }
.pc-tags-hint { font-size:0.75rem; font-weight:400; color:var(--ltextc); margin-left:6px; }
.pc-tags-input-row { position:relative; }
.pc-tag-input { width:100%; padding:7px 10px; font-size:0.9rem; border:1px solid var(--border); border-radius:4px; background:var(--bgc); color:var(--textc); box-sizing:border-box; }
.pc-tag-input:focus { outline:none; border-color:var(--accentc); }
.pc-tag-suggestions { position:absolute; top:100%; left:0; right:0; background:var(--bgc); border:1px solid var(--border); border-radius:0 0 4px 4px; z-index:10; display:none; }
.pc-suggestion { display:block; width:100%; padding:6px 10px; text-align:left; background:transparent; border:none; color:var(--textc); cursor:pointer; font-size:0.9rem; }
.pc-suggestion:hover { background:var(--alt-bg); }
.pc-tags-selected { display:flex; flex-wrap:wrap; gap:6px; min-height:0; }
.pc-tag-chip { display:inline-flex; align-items:center; gap:4px; padding:3px 10px; font-size:0.8rem; background:var(--bgc); border:1px solid var(--border); border-radius:20px; color:var(--textc); }
.pc-tag-any { background:var(--accentc); color:#fff; border-color:var(--accentc); opacity:0.7; }
.pc-tag-rm { background:transparent; border:none; color:inherit; cursor:pointer; font-size:0.75rem; line-height:1; padding:0; opacity:0.6; }
.pc-tag-rm:hover { opacity:1; }
.pc-tag-error { font-size:0.8rem; color:#e05; min-height:16px; }

.pc-editor-area { display:flex; flex-direction:column; gap:8px; }
.pc-drop-zone { border:2px dashed var(--border); border-radius:6px; padding:14px; text-align:center; color:var(--ltextc); font-size:0.9rem; transition:background 0.2s; cursor:default; }
.pc-drop-zone.pc-drop-over { background:var(--alt-bg); border-color:var(--accentc); }
.pc-panes { display:grid; grid-template-columns:1fr 1fr; gap:12px; }
@media(max-width:700px){.pc-panes{grid-template-columns:1fr;}}
.pc-textarea { min-height:260px; resize:vertical; padding:10px; font-family:monospace; font-size:0.95rem; border:1px solid var(--border); border-radius:4px; background:var(--bgc); color:var(--textc); }
.pc-preview { min-height:260px; padding:10px; border:1px solid var(--border); border-radius:4px; overflow-y:auto; background:var(--alt-bg); line-height:1.6; }
.pc-preview h2 { margin:12px 0 6px; }
.pc-preview ul.pc-list { padding-left:20px; }
.pc-preview ul.pc-checklist { list-style:none; padding-left:4px; }
.pc-preview a { color:var(--accentc); }
.pc-inline-img { max-width:100%; border-radius:4px; }
.pc-thumbs { display:flex; flex-wrap:wrap; gap:10px; }
.pc-thumb-wrap { display:flex; flex-direction:column; align-items:center; gap:4px; background:var(--alt-bg); border:1px solid var(--border); border-radius:6px; padding:6px; position:relative; width:120px; }
.pc-thumb-img { width:100px; height:80px; object-fit:cover; border-radius:4px; }
.pc-thumb-name { font-size:0.7rem; color:var(--ltextc); word-break:break-all; text-align:center; }
.pc-thumb-rm { position:absolute; top:4px; right:4px; background:transparent; border:none; cursor:pointer; font-size:0.85rem; color:var(--ltextc); }
.pc-thumb-rm:hover { color:red; }
.pc-footer { display:flex; flex-direction:column; gap:8px; }
.pc-progress-wrap { width:100%; background:var(--alt-bg); border:1px solid var(--border); border-radius:4px; overflow:hidden; position:relative; height:28px; }
.pc-progress-bar { height:100%; background:var(--accentc); transition:width 0.3s ease; width:0%; }
.pc-progress-label { position:absolute; top:50%; left:50%; transform:translate(-50%,-50%); font-size:0.8rem; color:var(--textc); white-space:nowrap; pointer-events:none; }
.pc-footer-row { display:flex; align-items:center; justify-content:space-between; }
.pc-status { font-size:0.9rem; color:var(--ltextc); }
.pc-submit-btn { padding:8px 24px; background:var(--accentc); color:#fff; border:none; border-radius:4px; cursor:pointer; font-size:1rem; }
.pc-submit-btn:disabled { opacity:0.5; cursor:not-allowed; }
.pc-submit-btn:not(:disabled):hover { opacity:0.85; }
.pc-feed-preview-label { font-size:0.8rem; font-weight:bold; color:var(--ltextc); text-transform:uppercase; letter-spacing:0.05em; margin-top:4px; }
.pc-feed-preview { margin:0; cursor:default; pointer-events:none; }
.pcfp-title-placeholder { color:var(--ltextc); font-style:italic; }
`;
