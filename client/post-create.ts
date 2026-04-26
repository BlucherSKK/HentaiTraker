import { HntWsConnection } from './ws';

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
        const url = src.startsWith('/') ? src : `/api/files/${escAttr(src)}`;
        return `<img src="${url}" alt="${escAttr(alt)}" class="pc-inline-img">`;
    })
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, label, href) =>
    `<a href="${escAttr(href)}" target="_blank" rel="noopener">${escMd(label)}</a>`
    );
}

function escMd(s: string): string { return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }
function escAttr(s: string): string { return s.replace(/"/g,'&quot;').replace(/</g,'&lt;'); }
function escapeRegex(s: string): string { return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

// ----- types -----

interface PendingImage { file: File; localUrl: string; name: string; }

// ----- PostCreatePage -----

export class PostCreatePage extends HTMLElement {
    public ws: HntWsConnection | undefined;

    private _images: PendingImage[] = [];
    private _busy = false;

    connectedCallback() { this._attachStyles(); this.render(); }

    private render() {
        this.innerHTML = `
        <div class="pc-wrap">
        <div class="pc-header">
        <span class="pc-title">Новый пост</span>
        <button class="pc-cancel-btn" id="pc-cancel">Отмена</button>
        </div>
        <input class="pc-input-title" id="pc-post-title" type="text" placeholder="Заголовок (необязательно)" maxlength="200">
        <div class="pc-editor-area">
        <div class="pc-drop-zone" id="pc-drop-zone">
        <span class="pc-drop-hint">Перетащите картинку сюда — она добавится в список и загрузится при публикации</span>
        </div>
        <div class="pc-panes">
        <textarea class="pc-textarea" id="pc-textarea" placeholder="## Заголовок&#10;- список&#10;- [ ] чеклист&#10;![alt](имя_файла.jpg)"></textarea>
        <div class="pc-preview" id="pc-preview"></div>
        </div>
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
        this._bindEvents();
    }

    private _bindEvents() {
        this.querySelector('#pc-cancel')?.addEventListener('click', () => this._cancel());
        this.querySelector('#pc-submit')?.addEventListener('click', () => this._submit());
        const ta = this.querySelector<HTMLTextAreaElement>('#pc-textarea');
        ta?.addEventListener('input', () => this._updatePreview());
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
    }

    private _updatePreview() {
        const ta = this.querySelector<HTMLTextAreaElement>('#pc-textarea');
        const preview = this.querySelector<HTMLElement>('#pc-preview');
        if (ta && preview) preview.innerHTML = renderMarkdown(ta.value);
    }

    private _addImage(file: File) {
        const allowed = ['image/jpeg','image/png','image/gif','image/webp'];
        if (!allowed.includes(file.type)) { this._setStatus('Только jpg/png/gif/webp'); return; }
        const localUrl = URL.createObjectURL(file);
        const pending: PendingImage = { file, localUrl, name: file.name };
        this._images.push(pending);
        this._renderThumb(pending);
        this._setStatus(`Добавлено: ${file.name}`);
    }

    private _renderThumb(img: PendingImage) {
        const thumbs = this.querySelector<HTMLElement>('#pc-thumbs');
        if (!thumbs) return;
        const wrap = document.createElement('div');
        wrap.className = 'pc-thumb-wrap';
        wrap.dataset.name = img.name;
        wrap.innerHTML = `
        <img src="${img.localUrl}" class="pc-thumb-img" alt="${img.name}">
        <span class="pc-thumb-name">${img.name}</span>
        <button class="pc-thumb-rm" title="Удалить">✕</button>`;
        wrap.querySelector('.pc-thumb-rm')?.addEventListener('click', () => {
            this._images = this._images.filter(i => i.name !== img.name);
            URL.revokeObjectURL(img.localUrl);
            wrap.remove();
        });
        thumbs.appendChild(wrap);
    }

    // ----- submit -----

    private async _submit() {
        if (this._busy) return;
        if (!this.ws) { this._setStatus('WS недоступен'); return; }

        const title   = (this.querySelector<HTMLInputElement>('#pc-post-title')?.value ?? '').trim();
        const content = (this.querySelector<HTMLTextAreaElement>('#pc-textarea')?.value ?? '').trim();
        if (!content) { this._setStatus('Текст поста не может быть пустым'); return; }

        this._busy = true;
        this._setSubmitEnabled(false);

        const uploadedUrls: string[] = [];
        const total = this._images.length;

        try {
            // ----- шаг 1: загружаем картинки -----
            for (let i = 0; i < total; i++) {
                const img = this._images[i];
                this._setProgress((i / (total + 1)) * 90, `Загрузка ${i + 1} из ${total}: ${img.file.name}`);

                const filename = await this._uploadOne(img.file);
                uploadedUrls.push(`/api/files/${filename}`);

                const ta = this.querySelector<HTMLTextAreaElement>('#pc-textarea');
                if (ta) {
                    ta.value = ta.value.replace(new RegExp(escapeRegex(img.name), 'g'), filename);
                    this._updatePreview();
                }
            }

            // ----- шаг 2: создаём пост -----
            this._setProgress(95, 'Публикуем пост…');
            const finalContent = (this.querySelector<HTMLTextAreaElement>('#pc-textarea')?.value ?? '').trim();
            const files = uploadedUrls.length > 0 ? JSON.stringify(uploadedUrls) : '';

            await new Promise<void>((resolve, reject) => {
                const offOk  = this.ws!.once('post_created', () => { offErr(); resolve(); });
                const offErr = this.ws!.once('error', (_ev, p) => { offOk(); reject(new Error(String(p.code))); });
                this.ws!.send('post_create', { title, content: finalContent, files })
                .catch(e => { offOk(); offErr(); reject(e); });
            });

            // ----- шаг 3: успех -----
            this._setProgress(100, 'Готово!');
            this._images.forEach(i => URL.revokeObjectURL(i.localUrl));
            this._images = [];
            setTimeout(() => this._navigate('feeds'), 700);

        } catch (err) {
            this._setStatus(`Ошибка: ${err}`);
            this._hideProgress();
            this._setSubmitEnabled(true);
            this._busy = false;
        }
    }

    private async _uploadOne(file: File): Promise<string> {
        if (!this.ws) throw new Error('no ws');
        const token = await new Promise<string>((resolve, reject) => {
            const off = this.ws!.once('upload_token', (_ev, p) => resolve(p.token as string));
            this.ws!.send('get_upload_token').catch(e => { off(); reject(e); });
            setTimeout(() => { off(); reject(new Error('token timeout')); }, 8000);
        });
        const fd = new FormData();
        fd.append('token', token);
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
`;
