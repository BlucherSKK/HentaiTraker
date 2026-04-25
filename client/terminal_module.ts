(function() {
    const ws = (window as any).__TERMINAL_WS__;
    const mount = document.getElementById('terminal-mount');
    const loader = document.getElementById('terminal-loader');
    if (!ws || !mount) return;

    // Скрыть кнопку загрузки
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

function appendLine(text: string, cls = 'output') {
    const div = document.createElement('div');
    div.className = `terminal-line terminal-${cls}`;
    div.textContent = text;
    out.appendChild(div);
    out.scrollTop = out.scrollHeight;
}

appendLine('[терминал] Подключено. Введите команду.', 'info');

// Получение ответов от сервера
ws.on('terminal_output', (_: string, p: Record<string, unknown>) => {
    appendLine(String(p.output ?? ''), 'output');
});
ws.on('terminal_error', (_: string, p: Record<string, unknown>) => {
    appendLine(String(p.message ?? 'Ошибка'), 'error');
});

// Отправка команды
input.addEventListener('keydown', async (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
        const cmd = input.value.trim();
        if (!cmd) return;
        history.unshift(cmd); histIdx = -1; input.value = '';
        appendLine('$ ' + cmd, 'input');
        try { await ws.send('terminal_cmd', { input: cmd }); }
        catch (err) { appendLine('Ошибка: ' + err, 'error'); }
    }
    if (e.key === 'ArrowUp')   { histIdx = Math.min(histIdx + 1, history.length - 1); input.value = history[histIdx] ?? ''; }
    if (e.key === 'ArrowDown') { histIdx = Math.max(histIdx - 1, -1); input.value = histIdx >= 0 ? history[histIdx] : ''; }
});

input.focus();
})();
