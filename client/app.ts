import { STYLES } from "./assets";
import { AuthPage } from "./auth";
import { Chats } from "./chats";
import { HntDataBase, init_test_db } from "./db";
import { get_header, get_nonlogin_dm_noty } from "./header";
import { Feed, HomeNav } from "./home";
import { AppNav } from "./nav";
import { HntWsConnection } from "./ws";
import { ProfilePage } from "./profile";
import { TerminalPage } from "./terminal";
import { PostCreatePage } from "./post-create";

declare global {
    interface Window {
        __VTNS__?: { pub_vtns: string; priv_vtns: string; ws: WebSocket; };
        __MODULE_STYLES__?: Record<string, string>;  // ← новое
    }
}

enum Tags {
    Any,
    Hentai
}

export interface User {
    name: string;
    id: string;
    token: string;
    roles: string;
    tagpool: Tags[];
}

type PageType = 'feeds' | 'projects' | 'settings' | 'login' | 'dm' | 'chats' | 'profile' | 'terminal' | 'post-create';

interface AppState {
    page: PageType;
    lastpage: PageType;
    user?: User;
    items: string[];
    init: boolean;
    db: HntDataBase;
    ws?: HntWsConnection;
}

customElements.define('app-feed',  Feed);
customElements.define('home-nav',  HomeNav);
customElements.define('app-nav',   AppNav);
customElements.define('app-chats', Chats);
customElements.define('app-auth',  AuthPage);
customElements.define('app-profile', ProfilePage);
customElements.define('app-terminal', TerminalPage);
customElements.define('post-create', PostCreatePage);

const App = {
    state: {
        page:     'feeds',
        lastpage: 'feeds',
        items:    ['Разработка на Rust', 'Настройка Arch Linux', 'Docker контейнеры'],
        init:     false,
        db:       init_test_db(),
    } as AppState,

    init(): void {
        this.initWs();
        this.applyStyles();
        this.render();
        this.initNavigation();

        (window as any).__registerModuleStyles = (id: string, css: string) => {
            this.registerModuleStyles(id, css);
        };
    },

    initWs(): void {
        const vtns = window.__VTNS__;
        if (!vtns) {
            console.warn('[App] ВТНС не найден — WebSocket недоступен');
            return;
        }
        this.state.ws = new HntWsConnection(vtns.ws, vtns);
        this.setupWsHandlers();
        console.log('[App] WebSocket подхвачен от лоадера');
    },

    applyStyles(): void {
        // Основные стили приложения
        if (!document.getElementById('app-styles')) {
            const style = document.createElement('style');
            style.id          = 'app-styles';
            style.textContent = STYLES;
            document.head.appendChild(style);
        }

        // Стили runtime-модулей (регистрируются через window.__MODULE_STYLES__)
        const moduleStyles = window.__MODULE_STYLES__ ?? {};
        for (const [moduleId, css] of Object.entries(moduleStyles)) {
            const styleId = `module-styles-${moduleId}`;
            if (document.getElementById(styleId)) continue;
            const style = document.createElement('style');
            style.id          = styleId;
            style.textContent = css;
            document.head.appendChild(style);
        }
    },

    // Добавить метод для регистрации стилей модулем в рантайме:
    registerModuleStyles(moduleId: string, css: string): void {
        window.__MODULE_STYLES__ = window.__MODULE_STYLES__ ?? {};
        window.__MODULE_STYLES__[moduleId] = css;

        // Если applyStyles уже отработал — инжектируем сразу
        const styleId = `module-styles-${moduleId}`;
        if (!document.getElementById(styleId)) {
            const style = document.createElement('style');
            style.id          = styleId;
            style.textContent = css;
            document.head.appendChild(style);
        }
    },

    setupWsHandlers(): void {
        const ws = this.state.ws;
        if (!ws) return;

        const onAuthSuccess = (_ev: string, payload: Record<string, unknown>) => {
            this.state.user = {
                name:    payload.username as string,
                id:      String(payload.user_id),
                token:   payload.pub_at   as string,
                roles: (payload.roles as string | null) ?? '',
                tagpool: [],
            };
            this.state.page = this.state.lastpage === 'login' ? 'feeds' : this.state.lastpage;
            this.state.init = false;
            history.replaceState({ page: this.state.page }, '', `/#${this.state.page}`);
            this.render();
        };

        ws.on('login_ok',    onAuthSuccess);
        ws.on('register_ok', onAuthSuccess);

        ws.on('error', (_ev, payload) => {
            console.error('[WS] Ошибка сервера:', payload);
        });

        ws.on('*', (event, payload) => {
            console.debug('[WS] event:', event, payload);
        });
    },

    render(): void {
        const root = document.getElementById('app');
        if (!root) return;

        if (!this.state.init) {
            root.innerHTML = `
            ${get_header("home", this.state.user)}
            <hero id="apphero"></hero>
            `;
            this.state.init = true;
        }

        const hero = document.getElementById('apphero');
        if (!hero) return;

        // При первом рендере создаём статичные страницы в DOM
        this.ensurePages(hero);

        // Скрываем все страницы, показываем нужную
        hero.querySelectorAll<HTMLElement>(':scope > .page-slot').forEach(el => {
            el.style.display = el.dataset.page === this.state.page ? '' : 'none';
        });

        // Обновляем nav во всех слотах
        hero.querySelectorAll<AppNav>('app-nav').forEach(nav => {
            nav.setAttribute('data-link', this.state.page);
            nav.setAttribute('data-user-roles', this.state.user?.roles || '');
        });

        // Передаём зависимости компонентам
        if (this.state.page === 'chats') {
            const el = hero.querySelector('app-chats') as Chats;
            if (el) { el.db = this.state.db; el.render(); }
        }
        if (this.state.page === 'profile') {
            const el = hero.querySelector('app-profile') as ProfilePage;
            if (el) { el.user = this.state.user; el.ws = this.state.ws; }
        }
        if (this.state.page === 'login') {
            const el = hero.querySelector('app-auth') as AuthPage;
            if (el) { el.ws = this.state.ws; }
        }
        if (this.state.page === 'terminal') {
            const termElem = root.querySelector('app-terminal') as TerminalPage;
            if (termElem) {
                window.__TERMINAL_WS__ = this.state.ws;
                termElem.ws = this.state.ws;
            }
        }
        if (this.state.page === 'post-create') {
            const el = hero.querySelector('app-post-create') as PostCreatePage;
            if (el) el.ws = this.state.ws;
        }


    },

    // Создаём все страницы один раз, потом только скрываем/показываем
    ensurePages(hero: HTMLElement): void {
        const pages: PageType[] = ['feeds', 'dm', 'chats', 'login', 'profile', 'terminal'];
        for (const page of pages) {
            if (hero.querySelector(`[data-page="${page}"]`)) continue;

            const slot = document.createElement('div');
            slot.className   = 'page-slot';
            slot.dataset.page = page;
            slot.style.display = 'none';
            slot.innerHTML = this.getPageTemplate(page);
            hero.appendChild(slot);
        }
    },

    getPageTemplate(page: PageType): string {
        const nav = `<app-nav data-link="${page}" data-user-roles="${this.state.user?.roles || ''}"></app-nav>`;
        switch (page) {
            case 'feeds':    return `${nav}<app-feed></app-feed>`;
            case 'dm':       return `${nav}${this.state.user ? '' : get_nonlogin_dm_noty()}`;
            case 'chats':    return `${nav}<app-chats></app-chats>`;
            case 'login':    return `<app-auth></app-auth>`;
            case 'profile':  return `${nav}<app-profile></app-profile>`;
            case 'terminal': return `${nav}<app-terminal></app-terminal>`;
            case 'post-create': return `${nav}<app-post-create></app-post-create>`;
            default:         return nav;
        }
    },

    initNavigation(): void {
        window.addEventListener('app-navigate', (e: Event) => {
            const detail = (e as CustomEvent).detail as { page: string };
            const targetPage = detail.page as PageType;
            if (targetPage && targetPage !== this.state.page) {
                this.state.lastpage = this.state.page;
                this.state.page     = targetPage;
                history.pushState({ page: targetPage }, '', `/#${targetPage}`);
                this.render();
            }
        });
        window.addEventListener('click', (e: MouseEvent) => {
            const target = e.target as HTMLElement;
            const link   = target.closest<HTMLElement>('[data-link]');
            if (link) {
                e.preventDefault();
                const targetPage = link.getAttribute('data-link') as PageType;
                if (targetPage && targetPage !== this.state.page) {
                    this.state.lastpage = this.state.page;
                    this.state.page     = targetPage;
                    history.pushState({ page: targetPage }, "", `/#${targetPage}`);
                    this.render();
                }
            }
        });

        window.addEventListener('popstate', (e: PopStateEvent) => {
            this.state.page = e.state?.page || 'feeds';
            this.render();
        });
    },

};

App.init();
