import { LOGO } from "./assets";
import { User } from "./app";



export function get_header(page: string, user?: User): string {
    return `
    <header class="header">
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
        <div class="auth-buttons">
        <a href="/login">Вход</a>
        <a href="/register">Регистрация</a>
        </div>
        `
    }
    </div>
    </header>
    `;
}
