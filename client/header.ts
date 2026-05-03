import { LOGO, SVG_LOGIN } from "./assets";
import { User } from "./app";
import { HntWsConnection } from "./ws";

// ----- ping indicator -----

const PING_DOT_ID  = 'ws-ping-dot';
const PING_ACTIVE  = 'ping-active';
let   _pingTimer: ReturnType<typeof setTimeout> | null = null;

export function bindPingIndicator(ws: HntWsConnection): void {
    ws.on('ping', () => {
        const dot = document.getElementById(PING_DOT_ID);
        if (!dot) return;

        if (_pingTimer) {
            clearTimeout(_pingTimer);
            _pingTimer = null;
        }

        dot.style.background  = '#4caf50';
        dot.style.boxShadow   = '0 0 5px #4caf50';

    _pingTimer = setTimeout(() => {
        dot.style.background = '';
        dot.style.boxShadow  = '';
        _pingTimer = null;
    }, 500);
    });
}

// ----- styles -----

const HEADER_STYLES = `
#ws-ping-dot {
width: 7px;
height: 7px;
border-radius: 50%;
background: #3a3a3a;
box-shadow: none;
display: inline-block;
flex-shrink: 0;
transition: background 1s ease, box-shadow 3s ease;
}
`;

function injectHeaderStyles(): void {
    if (document.getElementById('header-ping-styles')) return;
    const s = document.createElement('style');
    s.id          = 'header-ping-styles';
    s.textContent = HEADER_STYLES;
    document.head.appendChild(s);
}

// ----- template -----

export function get_header(page: string, user?: User): string {
    injectHeaderStyles();
    return `
    <header class="header">
    <div class="left-pane">
        <img class="logo-img pixelated" src="${LOGO}" alt="Logo"/>
        <div class="header-actions"></div>
    </div>
    <div class="right-pane">
        <span id="${PING_DOT_ID}" title="WebSocket"></span>
        ${user
            ? `<div class="user-container">
            <user-chip class="profile-btn" data-self data-name="${user.name}" data-link="profile"></user-chip>
            </div>`
            : `<a class="auth-btn" data-link="login">
            <img src="${SVG_LOGIN}"/>
            </a>`
        }
    </div>
    </header>
    `;
}

export function get_nonlogin_dm_noty(): string {
    console.log("get nonlogin noty");
    return `
    <div class="center-hero-noty">
    <div class="hero-noty">
    <a class="normal-noty-text">Просмотр личных сообщений доступен только авторизированным пользователям</a>
    <button class="btn-login" data-link="login">Вход</button>
    </div>
    </div>
    `;
}
