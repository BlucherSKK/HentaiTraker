import { HntWsConnection } from './ws';

type AuthTab = 'login' | 'register';

export class AuthPage extends HTMLElement {
    public ws: HntWsConnection | undefined;

    private _tab: AuthTab = 'login';

    connectedCallback() {
        this.render();
    }

    private render() {
        this.innerHTML = `
        <div class="auth-page">
        <div class="auth-card">
        <div class="auth-tabs">
        <button class="auth-tab ${this._tab === 'login' ? 'auth-tab--active' : ''}" data-tab="login">Войти</button>
        <button class="auth-tab ${this._tab === 'register' ? 'auth-tab--active' : ''}" data-tab="register">Зарегистрироваться</button>
        </div>
        ${this._tab === 'login' ? this.renderLogin() : this.renderRegister()}
        </div>
        </div>
        `;
        this.attachEvents();
    }

    private renderLogin(): string {
        return `
        <form class="auth-form" id="auth-form-login">
        <div class="auth-field">
        <label class="auth-label" for="login-username">Имя пользователя</label>
        <input class="auth-input" id="login-username" type="text" autocomplete="username" placeholder="Введите логин" required />
        </div>
        <div class="auth-field">
        <label class="auth-label" for="login-password">Пароль</label>
        <input class="auth-input" id="login-password" type="password" autocomplete="current-password" placeholder="Введите пароль" required />
        </div>
        <div class="auth-error" id="auth-error" hidden></div>
        <button class="auth-submit" type="submit">Войти</button>
        </form>
        `;
    }

    // ----- renderRegister(): добавить поле инвайта первым -----
    private renderRegister(): string {
        return `
        <form class="auth-form" id="auth-form-register">
        <div class="auth-field">
        <label class="auth-label" for="reg-invite">Инвайт-токен</label>
        <input class="auth-input" id="reg-invite" type="text" placeholder="Токен от администратора" required />
        </div>
        <div class="auth-field">
        <label class="auth-label" for="reg-username">Имя пользователя</label>
        <input class="auth-input" id="reg-username" type="text" autocomplete="username" placeholder="От 3 до 32 символов" required minlength="3" maxlength="32" />
        </div>
        <div class="auth-field">
        <label class="auth-label" for="reg-password">Пароль</label>
        <input class="auth-input" id="reg-password" type="password" autocomplete="new-password" placeholder="Минимум 6 символов" required minlength="6" />
        </div>
        <div class="auth-field">
        <label class="auth-label" for="reg-password2">Подтвердите пароль</label>
        <input class="auth-input" id="reg-password2" type="password" autocomplete="new-password" placeholder="Повторите пароль" required minlength="6" />
        </div>
        <div class="auth-error" id="auth-error" hidden></div>
        <button class="auth-submit" type="submit">Создать аккаунт</button>
        </form>
        `;
    }

    // ----- handleRegister(): читать инвайт и передавать -----
    private async handleRegister() {
        if (!this.ws) { this.showError('Нет соединения с сервером'); return; }

        const inviteToken = this.querySelector<HTMLInputElement>('#reg-invite')?.value.trim() ?? '';
        const username    = this.querySelector<HTMLInputElement>('#reg-username')?.value.trim() ?? '';
        const password    = this.querySelector<HTMLInputElement>('#reg-password')?.value ?? '';
        const password2   = this.querySelector<HTMLInputElement>('#reg-password2')?.value ?? '';

        if (!inviteToken)               { this.showError('Введите инвайт-токен'); return; }
        if (!username || !password || !password2) { this.showError('Заполните все поля'); return; }
        if (password !== password2)     { this.showError('Пароли не совпадают'); return; }
        if (password.length < 6)        { this.showError('Пароль должен быть минимум 6 символов'); return; }

        this.setLoading(true);

        const cleanup = this.ws.once('register_failed', (_ev, payload) => {
            this.setLoading(false);
            const code = payload.code as string;
            const messages: Record<string, string> = {
                invalid_invite_token: 'Неверный или уже использованный инвайт-токен',
                username_taken:       'Это имя уже занято',
                invalid_username:     'Имя должно быть от 3 до 32 символов',
                password_too_short:   'Пароль слишком короткий',
                db_error:             'Ошибка сервера, попробуйте позже',
            };
            this.showError(messages[code] ?? 'Ошибка регистрации');
        });

        try {
            await this.ws.register(username, password, inviteToken);
        } catch {
            cleanup();
            this.setLoading(false);
            this.showError('Ошибка соединения');
        }
    }

    private attachEvents() {
        this.querySelectorAll<HTMLButtonElement>('[data-tab]').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.preventDefault();
                e.stopPropagation();
                const tab = btn.getAttribute('data-tab') as AuthTab;
                if (tab !== this._tab) {
                    this._tab = tab;
                    this.render();
                }
            });
        });

        const loginForm = this.querySelector<HTMLFormElement>('#auth-form-login');
        if (loginForm) {
            loginForm.addEventListener('submit', (e) => {
                e.preventDefault();
                this.handleLogin();
            });
        }

        const regForm = this.querySelector<HTMLFormElement>('#auth-form-register');
        if (regForm) {
            regForm.addEventListener('submit', (e) => {
                e.preventDefault();
                this.handleRegister();
            });
        }
    }

    private showError(msg: string) {
        const el = this.querySelector<HTMLElement>('#auth-error');
        if (!el) return;
        el.textContent = msg;
        el.removeAttribute('hidden');
    }

    private setLoading(loading: boolean) {
        const btn = this.querySelector<HTMLButtonElement>('.auth-submit');
        if (!btn) return;
        btn.disabled = loading;
        btn.textContent = loading
        ? 'Подождите...'
        : this._tab === 'login' ? 'Войти' : 'Создать аккаунт';
    }

    private async handleLogin() {
        if (!this.ws) { this.showError('Нет соединения с сервером'); return; }

        const username = this.querySelector<HTMLInputElement>('#login-username')?.value.trim() ?? '';
        const password = this.querySelector<HTMLInputElement>('#login-password')?.value ?? '';
        if (!username || !password) { this.showError('Заполните все поля'); return; }

        this.setLoading(true);

        const cleanup = this.ws.once('login_failed', (_ev, payload) => {
            this.setLoading(false);
            const code = payload.code as string;
            const messages: Record<string, string> = {
                user_not_found: 'Пользователь не найден',
                wrong_password: 'Неверный пароль',
                db_error:       'Ошибка сервера, попробуйте позже',
            };
            this.showError(messages[code] ?? 'Ошибка входа');
        });

        try {
            await this.ws.login(username, password);
        } catch {
            cleanup();
            this.setLoading(false);
            this.showError('Ошибка соединения');
        }
    }

}
