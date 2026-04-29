// ----- admin-term/mod.ts -----

import { STYLE } from "./assets";

(window as any).__TERMINAL_INIT__ = function() {
    const ws     = (window as any).__TERMINAL_WS__;
    const mount  = document.getElementById('terminal-mount');
    const loader = document.getElementById('terminal-loader');
    if (!ws || !mount) return;

    if (typeof (window as any).__registerModuleStyles === 'function') {
        (window as any).__registerModuleStyles('terminal', STYLE);
    }

    if (loader) loader.style.display = 'none';

    // ----- state -----

    const STATE_KEY = '__TERMINAL_STATE__';
    let state: { history: string[]; lines: Array<{ text: string; color: string; marginLeft: string }> } =
    (window as any)[STATE_KEY] ?? { history: [], lines: [] };
    (window as any)[STATE_KEY] = state;

    let histIdx = -1;

    // ----- first render -----

    const alreadyMounted = mount.querySelector('.terminal-window') !== null;

    if (!alreadyMounted) {
        mount.innerHTML = `
        <div class="terminal-window">
        <div class="terminal-output" id="term-out"></div>
        <div class="terminal-input-row">
        <span class="terminal-prompt">$ </span>
        <input type="text" id="term-in" autocomplete="off" spellcheck="false"/>
        </div>
        </div>`;
    }

    const out   = document.getElementById('term-out')!;
    const input = document.getElementById('term-in') as HTMLInputElement;

    // ----- restore lines -----

    if (!alreadyMounted) {
        out.innerHTML = '';
        for (const ln of state.lines) {
            _appendLineDOM(out, ln.text, ln.color, ln.marginLeft);
        }
        if (state.lines.length === 0) {
            appendLine('[терминал] Подключено. Введите команду.', '#5ab0f7');
        }
        out.scrollTop = out.scrollHeight;
    }

    // ----- helpers -----

    function _appendLineDOM(container: HTMLElement, text: string, color: string, marginLeft: string) {
        const div = document.createElement('div');
        div.className = 'terminal-line';
        div.style.cssText = `color: ${color}; margin-left: ${marginLeft};`;
        div.textContent = text;
        container.appendChild(div);
    }

    function appendLine(text: string, color = 'var(--ltextc)', marginLeft = '0') {
        state.lines.push({ text, color, marginLeft });
        _appendLineDOM(out, text, color, marginLeft);
        out.scrollTop = out.scrollHeight;
    }

    // ----- ws listeners (re-register each time) -----

    ws.off('terminal_output');
    ws.off('terminal_error');

    ws.on('terminal_output', (_: string, p: Record<string, unknown>) => {
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
    });

    ws.on('terminal_error', (_: string, p: Record<string, unknown>) => {
        appendLine(String(p.message ?? 'Ошибка'), '#f75a5a');
    });

    // ----- input -----

    const oldInput = input.cloneNode(true) as HTMLInputElement;
    input.parentNode!.replaceChild(oldInput, input);
    const freshInput = document.getElementById('term-in') as HTMLInputElement;

    freshInput.addEventListener('keydown', async (e: KeyboardEvent) => {
        if (e.key === 'Enter') {
            const cmd = freshInput.value.trim();
            if (!cmd) return;
            state.history.unshift(cmd);
            histIdx = -1;
            freshInput.value = '';
            appendLine('$ ' + cmd, '#4ec94e');
            try { await ws.send('terminal_cmd', { input: cmd }); }
            catch (err) { appendLine('Ошибка: ' + err, '#f75a5a'); }
        }
        if (e.key === 'ArrowUp') {
            histIdx = Math.min(histIdx + 1, state.history.length - 1);
            freshInput.value = state.history[histIdx] ?? '';
        }
        if (e.key === 'ArrowDown') {
            histIdx = Math.max(histIdx - 1, -1);
            freshInput.value = histIdx >= 0 ? state.history[histIdx] : '';
        }
    });

    freshInput.focus();
};
