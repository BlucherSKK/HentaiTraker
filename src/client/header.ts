import { LOGO, SVG_LOGIN } from "./assets";
import { User } from "./app";



export function get_header(page: string, user?: User): string {
    return `
    <header class="header">
    <div class="right-pane">
        <img class="logo-img pixelated" src="${LOGO}" alt="Logo"/>

    <div class="header-actions">
    ${user
        ? `
        <div class="user-container">
        <span>${user.name}</span>
        <a href="/profile">Профиль</a>
        </div>
        `
        : `
        <a class="auth-btn" data-link="login">
            <img src="${SVG_LOGIN}"/>
        </a>
        `
    }
    </div>
    </header>
    `;
}

