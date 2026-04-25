import { HntWsConnection } from "./ws";  // ← обязательный импорт

// Объявление глобальных для TypeScript:
declare global {
    interface Window {
        __TERMINAL_WS__?: HntWsConnection;
    }
}

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
        <button id="load-terminal-btn">Загрузить терминал</button>
        </div>
        </div>`;
        this.querySelector('#load-terminal-btn')?.addEventListener('click', () => this.loadTerminal());
    }

    async loadTerminal() { /* fetch /terminal → Blob → script tag */ }
}
