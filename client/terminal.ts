import { HntWsConnection } from "./ws";

export class TerminalPage extends HTMLElement {
    ws?: HntWsConnection;
    private _loaded = false;

    connectedCallback() { this.render(); }

    render() {
        if (this._loaded) return;
        this.innerHTML = `
        <div class="terminal-wrapper">
        <div id="terminal-mount"></div>
        <div id="terminal-loader">
        <p>Терминал не загружен</p>
        <button id="load-terminal-btn" class="nav-btn">Загрузить терминал</button>
        </div>
        </div>`;
        this.querySelector('#load-terminal-btn')?.addEventListener('click', () => this.loadTerminal());
    }

    private async loadTerminal() {
        const btn = this.querySelector('#load-terminal-btn') as HTMLButtonElement;
        if (btn) { btn.disabled = true; btn.textContent = 'Загрузка...'; }

        try {
            // Регистрируем хелпер для модуля
            (window as any).__registerModuleStyles = (id: string, css: string) => {
                const styleId = `module-styles-${id}`;
                if (document.getElementById(styleId)) return;
                const style = document.createElement('style');
                style.id          = styleId;
                style.textContent = css;
                document.head.appendChild(style);
            };

            const resp = await fetch('/terminal');
            if (!resp.ok) throw new Error(`HTTP ${resp.status}`);

            const text = await resp.text();
            const blob = new Blob([text], { type: 'application/javascript' });
            const url  = URL.createObjectURL(blob);

            (window as any).__TERMINAL_WS__ = this.ws;

            const script = document.createElement('script');
            script.src = url;
            script.onerror = () => {
                URL.revokeObjectURL(url);
                if (btn) { btn.disabled = false; btn.textContent = 'Повторить'; }
            };
            script.onload = () => {
                URL.revokeObjectURL(url);
                this._loaded = true;
                const loader = this.querySelector('#terminal-loader') as HTMLElement;
                if (loader) loader.style.display = 'none';
            };
                document.head.appendChild(script);

        } catch (err) {
            if (btn) { btn.disabled = false; btn.textContent = 'Ошибка — повторить'; }
            console.error('[terminal] load error:', err);
        }
    }
}
