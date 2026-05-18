// ----- admin-term/metrics.ts -----

const METRICS_STYLE = `
.metrics-strip {
    display: flex;
    flex-direction: row;
    overflow-x: auto;
    overflow-y: hidden;
    gap: 1px;
    background: #0d0d0d;
    border: 1px solid var(--border);
    border-radius: 0.6em;
    box-sizing: border-box;
    height: 100%;
    margin-bottom: 1em;
}

.metrics-strip::-webkit-scrollbar        { height: 4px; }
.metrics-strip::-webkit-scrollbar-track  { background: #0d0d0d; }
.metrics-strip::-webkit-scrollbar-thumb  { background: #2a2a2a; border-radius: 2px; }

.metric-card {
    flex: 0 0 calc(100% / 4);
    min-width: 160px;
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 0.9em 1em 0.6em;
    border-right: 1px solid #1e1e1e;
    box-sizing: border-box;
    overflow: hidden;
}

.metric-card:last-child { border-right: none; }

.metric-label {
    font-family: 'Courier New', monospace;
    font-size: 0.65em;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    flex-shrink: 0;
}

.metric-value {
    font-family: 'Courier New', monospace;
    font-size: 1.8em;
    font-weight: bold;
    color: #e0e0e0;
    margin: 0.15em 0 0.4em;
    flex-shrink: 0;
    line-height: 1;
}

.metric-chart {
    flex: 1;
    width: 100%;
    min-height: 0;
    overflow: hidden;
}
`;

// ----- types -----

const HISTORY_LEN = 40;

type MetricKey =
| 'uptime_secs'
| 'connections_now'
| 'connections_peak'
| 'connections_total'
| 'users_online'
| 'messages_total'
| 'uploads_total';

interface MetricDef {
    key:   MetricKey;
    label: string;
    color: string;
}

interface MetricCardState {
    def:     MetricDef;
    history: number[];
    valueEl: HTMLElement;
    chartEl: HTMLElement;
}

// ----- config -----

const METRIC_DEFS: MetricDef[] = [
    { key: 'connections_now',   label: 'соединений сейчас',    color: '#5ab0f7' },
{ key: 'users_online',      label: 'пользователей онлайн', color: '#4ec94e' },
{ key: 'connections_peak',  label: 'пик соединений',       color: '#f7a25a' },
{ key: 'connections_total', label: 'всего соединений',     color: '#a25af7' },
{ key: 'messages_total',    label: 'сообщений отправлено', color: '#f75a5a' },
{ key: 'uploads_total',     label: 'загрузок файлов',      color: '#5af7e0' },
{ key: 'uptime_secs',       label: 'аптайм (сек)',         color: '#f7d95a' },
];

// ----- module state -----

let _cards:    MetricCardState[] = [];
let _handler:  ((ev: string, p: Record<string, unknown>) => void) | null = null;
let _interval: ReturnType<typeof setInterval> | null = null;

// ----- svg chart -----

function _renderSparkline(history: number[], color: string): string {
    if (history.length < 2) return '';

    const W   = 200;
    const H   = 60;
    const min = Math.min(...history);
    const max = Math.max(...history);
    const rng = max - min || 1;
    const pad = H * 0.1;

    const pts = history.map((v, i) => {
        const x = (i / (history.length - 1)) * W;
        const y = H - pad - ((v - min) / rng) * (H - pad * 2);
        return `${x.toFixed(2)},${y.toFixed(2)}`;
    });

    const linePts = pts.join(' ');
    const areaPts = `${pts.join(' ')} ${W},${H} 0,${H}`;

    return `<svg xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 ${W} ${H}"
    preserveAspectRatio="none"
    width="100%" height="100%">
    <polyline points="${areaPts}"
    fill="${color}" fill-opacity="0.12" stroke="none"/>
    <polyline points="${linePts}"
    fill="none" stroke="${color}" stroke-width="1.8"
    stroke-linejoin="round" stroke-linecap="round"
    vector-effect="non-scaling-stroke"/>
    </svg>`;
}

// ----- dom -----

function _buildStrip(container: HTMLElement): MetricCardState[] {
    const strip = document.createElement('div');
    strip.className = 'metrics-strip';

    const cards: MetricCardState[] = METRIC_DEFS.map(def => {
        const card  = document.createElement('div');
        card.className = 'metric-card';

        const label = document.createElement('div');
        label.className   = 'metric-label';
        label.textContent = def.label;

        const value = document.createElement('div');
        value.className   = 'metric-value';
        value.textContent = '—';

        const chart = document.createElement('div');
        chart.className = 'metric-chart';

        card.append(label, value, chart);
        strip.appendChild(card);

        return { def, history: [], valueEl: value, chartEl: chart };
    });

    container.appendChild(strip);
    return cards;
}

// ----- update -----

function _applySnapshot(p: Record<string, unknown>): void {
    for (const card of _cards) {
        const raw = p[card.def.key];
        if (raw === undefined) continue;

        const val = Number(raw);
        card.history.push(val);
        if (card.history.length > HISTORY_LEN) card.history.shift();

        card.valueEl.textContent = val.toString();
        card.chartEl.innerHTML   = _renderSparkline(card.history, card.def.color);
    }
}

// ----- ws -----

function _attachHandler(ws: any): void {
    if (_handler) ws.off('metrics_snapshot', _handler);

    _handler = (_: string, p: Record<string, unknown>) => _applySnapshot(p);
    ws.on('metrics_snapshot', _handler);
}

function _startPolling(ws: any): void {
    if (_interval) clearInterval(_interval);

    const poll = () => ws.send('metrics_get', {}).catch(() => {});
    poll();
    _interval = setInterval(poll, 3000);
}

// ----- public -----

export function initMetrics(ws: any): void {
    const container = document.getElementById('metrics');
    if (!container) return;

    if (typeof (window as any).__registerModuleStyles === 'function') {
        (window as any).__registerModuleStyles('metrics', METRICS_STYLE);
    }

    if (!container.querySelector('.metrics-strip')) {
        _cards = _buildStrip(container);
    }

    _attachHandler(ws);
    _startPolling(ws);
}

export function destroyMetrics(ws: any): void {
    if (_interval) { clearInterval(_interval); _interval = null; }
    if (_handler)  { ws.off('metrics_snapshot', _handler); _handler = null; }
    _cards = [];
}
