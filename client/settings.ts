import { HntWsConnection } from './ws';
import { toast } from './toast';
import { getSettings, updateSettings, applySettings, UserSettings } from './store';

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
        </div>
        <button class="settings-save-btn" id="settings-save-btn">Сохранить</button>
        </div>
        `;

        this.querySelector('#settings-save-btn')?.addEventListener('click', () => this.save());
    }

    private async save() {
        const select = this.querySelector<HTMLSelectElement>('#toast-position-select');
        if (!select) return;

        const patch: Partial<UserSettings> = {
            toast_position: select.value as UserSettings['toast_position'],
        };

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
