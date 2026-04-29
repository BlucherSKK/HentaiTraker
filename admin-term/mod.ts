// ----- admin-term/mod.ts -----

import { STYLE } from "./assets";

let _termOut:   HTMLElement | null = null;
let _termInput: HTMLInputElement | null = null;
let _state: {
    history:  string[];
    lines:    Array<{ text: string; color: string; marginLeft: string }>;
    handlers: { out: (e: string, p: Record<string, unknown>) => void; err: (e: string, p: Record<string, unknown>) => void } | null;
} = { history: [], lines: [], handlers: null };

(window as any).__TERMINAL_INIT__ = function() {
    const ws    = (window as any).__TERMINAL_WS__;
    const mount = document.getElementById('terminal-mount');
    if (!ws || !mount) return;

    if (typeof (window as any).__registerModuleStyles === 'function') {
        (window as any).__registerModuleStyles('terminal', STYLE);
    }

    const loader = document.getElementById('terminal-loader');
    if (loader) loader.style.display = 'none';

    // ----- build UI once -----

    if (!mount.querySelector('.terminal-window')) {
        mount.innerHTML = `
        <div class="terminal-window">
        <div class="terminal-output" id="term-out"></div>
        <div class="terminal-input-row">
        <span class="terminal-prompt">$ </span>
        <input type="text" id="term-in" autocomplete="off" spellcheck="false"/>
        </div>
        </div>`;

        _termOut   = document.getElementById('term-out') as HTMLElement;
        _termInput = document.getElementById('term-in') as HTMLInputElement;

        for (const ln of _state.lines) {
            _appendLineDOM(_termOut, ln.text, ln.color, ln.marginLeft);
        }
        if (_state.lines.length === 0) {
            appendLine('[терминал] Подключено. Введите команду.', '#5ab0f7');
        }
        _termOut.scrollTop = _termOut.scrollHeight;

        _termInput.addEventListener('keydown', async (e: KeyboardEvent) => {
            let histIdx = -1;
            if (e.key === 'Enter') {
                const cmd = _termInput!.value.trim();
                if (!cmd) return;
                _state.history.unshift(cmd);
                _termInput!.value = '';
                appendLine('$ ' + cmd, '#4ec94e');
                try { await ws.send('terminal_cmd', { input: cmd }); }
                catch (err) { appendLine('Ошибка: ' + err, '#f75a5a'); }
            }
            if (e.key === 'ArrowUp') {
                histIdx = Math.min(histIdx + 1, _state.history.length - 1);
                _termInput!.value = _state.history[histIdx] ?? '';
            }
            if (e.key === 'ArrowDown') {
                histIdx = Math.max(histIdx - 1, -1);
                _termInput!.value = histIdx >= 0 ? _state.history[histIdx] : '';
            }
        });
    } else {
        _termOut   = document.getElementById('term-out') as HTMLElement;
        _termInput = document.getElementById('term-in') as HTMLInputElement;
    }

    _termInput?.focus();

    // ----- ws listeners — снимаем старые, вешаем новые -----

    if (_state.handlers) {
        ws.off('terminal_output', _state.handlers.out);
        ws.off('terminal_error',  _state.handlers.err);
    }

    const outHandler = (_: string, p: Record<string, unknown>) => {
        const content = String(p.output ?? '');
        if (content.includes('#NL#')) {
            for (const segment of content.split('#NL#')) {
                let line  = segment;
                let color = 'var(--ltextc)';
                if (line.includes('#C#')) {
                    const parts = line.split('#C#');
                    color = parts[1];
                    line  = line.replace(/#C#.*?#C#/g, '');
                }
                const marginLeft = line.startsWith('#T#') ? '4rem' : '0';
                appendLine(line.replace('#T#', ''), color, marginLeft);
            }
        } else {
            appendLine(content);
        }
    };

    const errHandler = (_: string, p: Record<string, unknown>) => {
        appendLine(String(p.message ?? 'Ошибка'), '#f75a5a');
    };

    _state.handlers = { out: outHandler, err: errHandler };
    ws.on('terminal_output', outHandler);
    ws.on('terminal_error',  errHandler);
};

function _appendLineDOM(container: HTMLElement, text: string, color: string, marginLeft: string) {
    const div = document.createElement('div');
    div.className    = 'terminal-line';
    div.style.cssText = `color: ${color}; margin-left: ${marginLeft};`;
    div.textContent  = text;
    container.appendChild(div);
}

function appendLine(text: string, color = 'var(--ltextc)', marginLeft = '0') {
    _state.lines.push({ text, color, marginLeft });
    if (_termOut) {
        _appendLineDOM(_termOut, text, color, marginLeft);
        _termOut.scrollTop = _termOut.scrollHeight;
    }
}
