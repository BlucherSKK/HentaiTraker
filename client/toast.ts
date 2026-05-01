import { getSettings } from './store';

export type ToastKind = 'info' | 'success' | 'warn' | 'error';
export type ToastEdge = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

export interface ToastOptions {
    kind?:     ToastKind;
    edge?:     ToastEdge;
    duration?: number;
    title?:    string;
}

// ----- container registry -----

const CONTAINER_ID_PREFIX = 'toast-container-';

function getContainer(edge: ToastEdge): HTMLElement {
    const id = `${CONTAINER_ID_PREFIX}${edge}`;
    let el = document.getElementById(id);
    if (!el) {
        el = document.createElement('div');
        el.id        = id;
        el.className = `toast-container toast-container--${edge}`;
        document.body.appendChild(el);
    }
    return el;
}

// ----- styles -----

const TOAST_STYLES = `
.toast-container {
    position: fixed;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
    max-width: 320px;
}
.toast-container--top-left     { top: 150px;  left: 1vw; }
.toast-container--top-right    { top: 150px;  right: 1vw; }
.toast-container--bottom-left  { bottom: 16px; left: 16px; }
.toast-container--bottom-right { bottom: 16px; right: 16px; }

.toast {
    pointer-events: all;
    padding: 10px 14px;
    font-size: 14px;
    line-height: 1.4;
    color: #fff;
    box-shadow: 0 4px 12px rgba(0,0,0,.35);
    display: flex;
    flex-direction: column;
    gap: 2px;
    opacity: 0;
    transform: translateY(-6px);
    transition: opacity .6000s ease, transform .8000s ease;
    cursor: pointer;
    word-break: break-word;
    border: 1.5px darkgray solid;
}
.toast.toast--visible {
    opacity: 1;
    transform: translateY(0);
}
.toast.toast--hiding {
    opacity: 0;
    transform: translateY(-6px);
}
.toast--info    { background: #1e6fbb; }
.toast--success { background: #2e7d32; }
.toast--warn    { background: #b45309; }
.toast--error   { background: #b71c1c; }

.toast-title {
    font-weight: 700;
    font-size: 13px;
    opacity: .85;
}
`;

function injectToastStyles(): void {
    if (document.getElementById('toast-styles')) return;
    const s = document.createElement('style');
    s.id          = 'toast-styles';
    s.textContent = TOAST_STYLES;
    document.head.appendChild(s);
}

// ----- core -----

export function toast(message: string, options: ToastOptions = {}): void {
    injectToastStyles();

    const {
        kind     = 'info',
        edge     = getSettings().toast_position,
        duration = 3500,
        title,
    } = options;

    const container = getContainer(edge);

    const el = document.createElement('div');
    el.className = `toast toast--${kind}`;
    el.innerHTML = [
        title ? `<span class="toast-title">${title}</span>` : '',
        `<span>${message}</span>`,
    ].join('');

    const dismiss = () => hide(el);
    el.addEventListener('click', dismiss);

    container.appendChild(el);

    requestAnimationFrame(() => {
        requestAnimationFrame(() => el.classList.add('toast--visible'));
    });

    const timer = setTimeout(dismiss, duration);
    el.addEventListener('click', () => clearTimeout(timer), { once: true });
}

function hide(el: HTMLElement): void {
    if (el.classList.contains('toast--hiding')) return;
    el.classList.remove('toast--visible');
    el.classList.add('toast--hiding');
    el.addEventListener('transitionend', () => el.remove(), { once: true });
}

// ----- shortcuts -----

export const toastInfo    = (msg: string, o?: Omit<ToastOptions, 'kind'>) => toast(msg, { ...o, kind: 'info'    });
export const toastSuccess = (msg: string, o?: Omit<ToastOptions, 'kind'>) => toast(msg, { ...o, kind: 'success' });
export const toastWarn    = (msg: string, o?: Omit<ToastOptions, 'kind'>) => toast(msg, { ...o, kind: 'warn'    });
export const toastError   = (msg: string, o?: Omit<ToastOptions, 'kind'>) => toast(msg, { ...o, kind: 'error'   });
