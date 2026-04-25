import { HntWsConnection } from "./ws";

declare global {
    interface Window {
        __TERMINAL_WS__?: HntWsConnection;
        __TERMINAL_INIT__?: () => void;
    }
}

export class TerminalPage extends HTMLElement {
    ws?: HntWsConnection;
    private _loaded = false;

    connectedCallback() {
        this.render();
        this.loadTerminal();
    }

    render() {
        this.innerHTML = `
        <div class="terminal-wrapper">
        <div id="terminal-mount"></div>
        <div id="terminal-loader">
        <div class="loader"></div>
        <p>Загрузка терминала…</p>
        </div>
        </div>`;
    }

    async loadTerminal() {
        if (this._loaded) return;
        this._loaded = true;

        // ws может прийти либо через атрибут (app.ts), либо уже висеть на window
        const ws = this.ws ?? window.__TERMINAL_WS__;
        if (!ws) {
            this.showError('WebSocket недоступен');
            return;
        }

        // Гарантируем что window.__TERMINAL_WS__ выставлен для terminal_module
        window.__TERMINAL_WS__ = ws;

        if (window.__TERMINAL_INIT__) {
            // Модуль уже загружен — просто переинициализируем UI в новый DOM
            window.__TERMINAL_INIT__();
            this.hideLoader();
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

            this.hideLoader();

        } catch (err) {
            this.showError(`Ошибка загрузки: ${err}`);
        }
    }

    private hideLoader() {
        const loader = this.querySelector<HTMLElement>('#terminal-loader');
        if (loader) loader.style.display = 'none';
    }

    private showError(msg: string) {
        const loader = this.querySelector('#terminal-loader');
        if (loader) loader.innerHTML = `<p style="color:#f87171">⚠ ${msg}</p>`;
    }
}
