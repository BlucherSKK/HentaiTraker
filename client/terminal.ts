// ----- client/terminal.ts -----

import { HntWsConnection } from "./ws";

declare global {
    interface Window {
        __TERMINAL_WS__?: HntWsConnection;
        __TERMINAL_INIT__?: () => void;
    }
}

export class TerminalPage extends HTMLElement {
    private _ws?: HntWsConnection;
    private _started = false;

    get ws(): HntWsConnection | undefined { return this._ws; }

    set ws(val: HntWsConnection | undefined) {
        this._ws = val;
        if (val && this.isConnected) {
            this._start();
        }
    }

    connectedCallback() {
        this._renderShell();
        if (this._ws) {
            this._start();
        }
    }

    render() {}

    private _renderShell() {
        this.innerHTML = `
        <div class="terminal-wrapper">
        <div id="terminal-mount"></div>
        <div id="terminal-loader">
        <div class="loader"></div>
        <p>Загрузка терминала…</p>
        </div>
        </div>`;
    }

    private async _start() {
        await Promise.resolve();

        if (this._started) {
            if (window.__TERMINAL_INIT__) {
                window.__TERMINAL_WS__ = this._ws;
                window.__TERMINAL_INIT__();
                this._hideLoader();
            }
            return;
        }
        this._started = true;

        window.__TERMINAL_WS__ = this._ws;

        if (window.__TERMINAL_INIT__) {
            window.__TERMINAL_INIT__();
            this._hideLoader();
            return;
        }

        try {
            const resp = await fetch('/terminal');
            if (!resp.ok) throw new Error(`HTTP ${resp.status}`);

            const blob = await resp.blob();
            const url  = URL.createObjectURL(blob);

            await new Promise<void>((resolve, reject) => {
                const script   = document.createElement('script');
                script.src     = url;
                script.onload  = () => { URL.revokeObjectURL(url); resolve(); };
                script.onerror = () => { URL.revokeObjectURL(url); reject(new Error('Ошибка скрипта')); };
                document.body.appendChild(script);
            });

            this._hideLoader();

        } catch (err) {
            this._showError(`Ошибка загрузки: ${err}`);
        }
    }

    private _hideLoader() {
        const loader = this.querySelector<HTMLElement>('#terminal-loader');
        if (loader) loader.style.display = 'none';
    }

    private _showError(msg: string) {
        const loader = this.querySelector('#terminal-loader');
        if (loader) loader.innerHTML = `<p style="color:#f87171">⚠ ${msg}</p>`;
    }
}
