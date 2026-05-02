import { HntWsConnection } from './ws';
import { toast, playToastSound } from './toast';
import { getSettings, updateSettings, UserSettings, ToastSound } from './store';

export { applySettings } from './store';

// ----- styles -----

const SETTINGS_STYLES = `
.settings-wrap {
    max-width: 520px;
    margin: 40px auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 24px;
    color: var(--textc);
}
.settings-title {
    font-size: 1.3rem;
    font-weight: bold;
}
.settings-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: var(--alt-bg);
    border: 1px solid var(--border);
    border-radius: 8px;
}
.settings-section-title {
    font-size: 0.85rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--ltextc);
}
.settings-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
}
.settings-label {
    font-size: 0.95rem;
}
.settings-select {
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bgc);
    color: var(--textc);
    font-size: 0.9rem;
    cursor: pointer;
}
.settings-sound-controls {
    display: flex;
    align-items: center;
    gap: 8px;
}
.settings-preview-btn {
    padding: 5px 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bgc);
    color: var(--textc);
    font-size: 0.85rem;
    cursor: pointer;
}
.settings-preview-btn:hover { opacity: 0.75; }
.settings-save-btn {
    align-self: flex-end;
    padding: 8px 24px;
    background: var(--accentc);
    color: #fff;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
}
.settings-save-btn:hover { opacity: 0.85; }
.settings-save-btn:disabled { opacity: 0.5; cursor: not-allowed; }
`;

function injectStyles(): void {
    if (document.getElementById('settings-styles')) return;
    const s = document.createElement('style');
    s.id          = 'settings-styles';
    s.textContent = SETTINGS_STYLES;
    document.head.appendChild(s);
}

// ----- helpers -----

type SoundRow = { id: string; label: string; key: keyof UserSettings };

const SOUND_ROWS: SoundRow[] = [
    { id: 'sound-info',    label: 'Звук — info',    key: 'toast_sound_info'    },
{ id: 'sound-success', label: 'Звук — success', key: 'toast_sound_success' },
{ id: 'sound-warn',    label: 'Звук — warn',    key: 'toast_sound_warn'    },
{ id: 'sound-error',   label: 'Звук — error',   key: 'toast_sound_error'   },
];

function soundOptions(current: ToastSound): string {
    const opts: { value: ToastSound; label: string }[] = [
        { value: 'none',  label: 'Без звука' },
        { value: 'soft',  label: 'Мягкий'    },
        { value: 'sharp', label: 'Резкий'    },
    ];
    return opts.map(o =>
    `<option value="${o.value}" ${current === o.value ? 'selected' : ''}>${o.label}</option>`
    ).join('');
}

function soundRow(row: SoundRow, current: ToastSound): string {
    return `
    <div class="settings-row">
    <span class="settings-label">${row.label}</span>
    <div class="settings-sound-controls">
    <select class="settings-select" id="${row.id}-select">
    ${soundOptions(current)}
    </select>
    <button class="settings-preview-btn" id="${row.id}-preview">▶</button>
    </div>
    </div>`;
}

// ----- component -----

export class SettingsPage extends HTMLElement {
    public ws?: HntWsConnection;

    connectedCallback() {
        injectStyles();
        this.render();
    }

    render() {
        const cur = getSettings();

        this.innerHTML = `
        <div class="settings-wrap">
        <div class="settings-title">Настройки</div>
        <div class="settings-section">
        <div class="settings-section-title">Уведомления</div>
        <div class="settings-row">
        <span class="settings-label">Положение уведомлений</span>
        <select class="settings-select" id="toast-position-select">
        <option value="top-left"     ${cur.toast_position === 'top-left'     ? 'selected' : ''}>Сверху слева</option>
        <option value="top-right"    ${cur.toast_position === 'top-right'    ? 'selected' : ''}>Сверху справа</option>
        <option value="bottom-left"  ${cur.toast_position === 'bottom-left'  ? 'selected' : ''}>Снизу слева</option>
        <option value="bottom-right" ${cur.toast_position === 'bottom-right' ? 'selected' : ''}>Снизу справа</option>
        </select>
        </div>
        ${SOUND_ROWS.map(row => soundRow(row, cur[row.key] as ToastSound)).join('')}
        </div>
        <button class="settings-save-btn" id="settings-save-btn">Сохранить</button>
        </div>
        `;

        SOUND_ROWS.forEach(row => {
            this.querySelector(`#${row.id}-preview`)?.addEventListener('click', () => {
                const sel = this.querySelector<HTMLSelectElement>(`#${row.id}-select`);
                if (sel) playToastSound(sel.value as ToastSound);
            });
        });

        this.querySelector('#settings-save-btn')?.addEventListener('click', () => this.save());
    }

    private async save() {
        const posSelect = this.querySelector<HTMLSelectElement>('#toast-position-select');
        if (!posSelect) return;

        const patch: Partial<UserSettings> = {
            toast_position: posSelect.value as UserSettings['toast_position'],
        };

        SOUND_ROWS.forEach(row => {
            const sel = this.querySelector<HTMLSelectElement>(`#${row.id}-select`);
            if (sel) (patch as any)[row.key] = sel.value as ToastSound;
        });

            updateSettings(patch);

            const btn = this.querySelector<HTMLButtonElement>('#settings-save-btn');
            if (btn) btn.disabled = true;

            if (!this.ws) {
                toast('Нет соединения', { kind: 'error' });
                if (btn) btn.disabled = false;
                return;
            }

            const raw = JSON.stringify(getSettings());

            const unsubOk = this.ws.once('settings_saved', () => {
                toast('Настройки сохранены', { kind: 'success' });
                if (btn) btn.disabled = false;
            });

                const unsubErr = this.ws.once('error', (_ev, payload) => {
                    toast(`Ошибка: ${payload.code}`, { kind: 'error' });
                    if (btn) btn.disabled = false;
                });

                    try {
                        await this.ws.send('settings_update', { settings: raw });
                    } catch (err) {
                        unsubOk();
                        unsubErr();
                        toast(`Ошибка: ${(err as Error).message}`, { kind: 'error' });
                        if (btn) btn.disabled = false;
                    }
    }
}
