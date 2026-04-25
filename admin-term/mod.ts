import { STYLE } from "./assets";

(window as any).__TERMINAL_INIT__ = function() {
    const ws    = (window as any).__TERMINAL_WS__;
    const mount = document.getElementById('terminal-mount');
    const loader = document.getElementById('terminal-loader');
    if (!ws || !mount) return;

    // ── Стили модуля ────────────────────────────────────────────────────
    if (typeof (window as any).__registerModuleStyles === 'function') {
        (window as any).__registerModuleStyles('terminal', STYLE);
    }

    // ── UI ───────────────────────────────────────────────────────────────
    if (loader) loader.style.display = 'none';

    // Рендер UI терминала
mount.innerHTML = `
<div class="terminal-window">
<div class="terminal-output" id="term-out"></div>
<div class="terminal-input-row">
<span class="terminal-prompt">$ </span>
<input type="text" id="term-in" autocomplete="off" spellcheck="false"/>
</div>
</div>`;

const out = document.getElementById('term-out')!;
const input = document.getElementById('term-in') as HTMLInputElement;
const history: string[] = [];
let histIdx = -1;

function appendLine(text: string, color = 'std', margin_left = '0') {
    const div = document.createElement('div');
    div.className = `terminal-line`;
    if(color == 'std'){ color = 'var(--ltextc)'}
    div.style = `color: ${color}; margin-left: ${margin_left};`
    div.textContent = text;
    out.appendChild(div);
    out.scrollTop = out.scrollHeight;
}

appendLine('[терминал] Подключено. Введите команду.', 'info');


ws.on('terminal_output', (_: string, p: Record<string, unknown>) => {
    let content = String(p.output ?? '');
    if (content.includes('#NL#')) {
        let p_content = content.split('#NL#');
        p_content.map((line: string) => {
            let color = 'std';
            if(line.includes('#C#')){
                let arr = line.split('#C#');
                color = arr[1];
                line = line.replace(/#C#.*?#C#/g, '');
            }
            if(line.includes('#T#')){
                appendLine(line.replace('#T#', ''), color, '4rem');
            } else {
                appendLine(line, color);
            }
        })
    } else {
        appendLine(String(p.output ?? ''));
    }
});
ws.on('terminal_error', (_: string, p: Record<string, unknown>) => {
    appendLine(String(p.message ?? 'Ошибка'), 'red');
});

// Отправка команды
input.addEventListener('keydown', async (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
        const cmd = input.value.trim();
        if (!cmd) return;
        history.unshift(cmd); histIdx = -1; input.value = '';
        appendLine('$ ' + cmd, 'green');
        try { await ws.send('terminal_cmd', { input: cmd }); }
        catch (err) { appendLine('Ошибка: ' + err, 'red'); }
    }
    if (e.key === 'ArrowUp')   { histIdx = Math.min(histIdx + 1, history.length - 1); input.value = history[histIdx] ?? ''; }
    if (e.key === 'ArrowDown') { histIdx = Math.max(histIdx - 1, -1); input.value = histIdx >= 0 ? history[histIdx] : ''; }
});

input.focus();
};

(window as any).__TERMINAL_INIT__();
