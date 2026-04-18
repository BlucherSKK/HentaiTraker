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

export function get_nonlogin_dm_noty(): string {
    console.log("get nonlogin noty");
    return `
    <div class="center-hero-noty">
        <div class="hero-noty">
            <a class="normal-noty-text">Просмотр личных сообщений доступен только авторизированным пользователям</a>
            <button class="btn-login" data-link="login">Вход</button>
        </div>
    </div>
    `
}
