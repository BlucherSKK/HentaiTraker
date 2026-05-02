import { STYLES } from "./assets";
import { AuthPage } from "./auth";
import { Chats } from "./chats";
import { HntDataBase, init_test_db } from "./db";
import { bindPingIndicator, get_header, get_nonlogin_dm_noty } from "./header";
import { Feed, HomeNav } from "./home";
import { AppNav } from "./nav";
import { HntWsConnection } from "./ws";
import { ProfilePage } from "./profile";
import { TerminalPage } from "./terminal";
import { PostCreatePage } from "./post-create";
import { SidebarNews } from './sidebar-news';
import { SettingsPage, applySettings } from './settings';
import { PostPage } from "./post-page";

declare global {
    interface Window {
        __VTNS__?: { pub_vtns: string; priv_vtns: string; ws: WebSocket; };
        __MODULE_STYLES__?: Record<string, string>;
    }
}

enum Tags { Any, Hentai }

export interface User {
    name:     string;
    id:       string;
    token:    string;
    roles:    string;
    tagpool:  Tags[];
    settings: string | null;
}

type PageType = 'feeds' | 'projects' | 'settings' | 'login' | 'dm' | 'chats' | 'profile' | 'terminal' | 'post-create' | 'post-page';

interface AppState {
    page:     PageType;
    lastpage: PageType;
    user?:    User;
    items:    string[];
    init:     boolean;
    db:       HntDataBase;
    ws?:      HntWsConnection;
    postId?:  number;
}

// ----- custom elements -----

customElements.define('app-feed',         Feed);
customElements.define('home-nav',         HomeNav);
customElements.define('app-nav',          AppNav);
customElements.define('app-chats',        Chats);
customElements.define('app-auth',         AuthPage);
customElements.define('app-profile',      ProfilePage);
customElements.define('app-terminal',     TerminalPage);
customElements.define('app-post-create',  PostCreatePage);
customElements.define('app-sidebar-news', SidebarNews);
customElements.define('app-settings',     SettingsPage);
customElements.define('app-post-page', PostPage);

// ----- App -----

const App = {
    state: {
        page:     'feeds',
        lastpage: 'feeds',
        items:    [],
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
        if (!document.getElementById('app-styles')) {
            const style = document.createElement('style');
            style.id          = 'app-styles';
            style.textContent = STYLES;
            document.head.appendChild(style);
        }

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

    registerModuleStyles(moduleId: string, css: string): void {
        window.__MODULE_STYLES__ = window.__MODULE_STYLES__ ?? {};
        window.__MODULE_STYLES__[moduleId] = css;

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

        bindPingIndicator(ws);

        const onAuthSuccess = (_ev: string, payload: Record<string, unknown>) => {
            const rawSettings = (payload.settings as string | null) ?? null;
            applySettings(rawSettings);

            this.state.user = {
                name:     payload.username as string,
                id:       String(payload.user_id),
                token:    payload.pub_at   as string,
                roles:   (payload.roles as string | null) ?? '',
                tagpool:  [],
                settings: rawSettings,
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

        this.ensurePages(hero);

        hero.querySelectorAll<HTMLElement>(':scope > .page-slot').forEach(el => {
            el.style.display = el.dataset.page === this.state.page ? '' : 'none';
        });

        hero.querySelectorAll<AppNav>('app-nav').forEach(nav => {
            nav.setAttribute('data-link',       this.state.page);
            nav.setAttribute('data-user-roles', this.state.user?.roles || '');
        });

        // ----- передача зависимостей -----

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
            if (el) el.ws = this.state.ws;
        }
        if (this.state.page === 'terminal') {
            const el = hero.querySelector('app-terminal') as TerminalPage;
            if (el) {
                window.__TERMINAL_WS__ = this.state.ws;
                el.ws = this.state.ws;
            }
        }
        if (this.state.page === 'post-create') {
            const el = hero.querySelector('app-post-create') as PostCreatePage;
            if (el) el.ws = this.state.ws;
        }
        if (this.state.page === 'settings') {
            const el = hero.querySelector('app-settings') as SettingsPage;
            if (el) el.ws = this.state.ws;
        }
        if (this.state.page === 'post-page') {
            const el = hero.querySelector('app-post-page') as PostPage;
            if (el) el.postId = this.state.postId ?? null;
        }
    },

    // ----- page slots -----

    ensurePages(hero: HTMLElement): void {
        const pages: PageType[] = ['feeds', 'dm', 'chats', 'login', 'profile', 'terminal', 'post-create', 'settings', 'post-page'];
        for (const page of pages) {
            if (hero.querySelector(`[data-page="${page}"]`)) continue;

            const slot = document.createElement('div');
            slot.className    = 'page-slot';
            slot.dataset.page = page;
            slot.style.display = 'none';
            slot.innerHTML = this.getPageTemplate(page);
            hero.appendChild(slot);
        }
    },

    getPageTemplate(page: PageType): string {
        const nav = `<app-nav data-link="${page}" data-user-roles="${this.state.user?.roles || ''}" data-user-id="${this.state.user?.id || ''}"></app-nav>`;
        switch (page) {
            case 'feeds': return `
                ${nav}
                <div class="feeds-layout">
                <app-sidebar-news></app-sidebar-news>
                <app-feed></app-feed>
                </div>`;
            case 'dm':          return `${nav}${this.state.user ? '' : get_nonlogin_dm_noty()}`;
            case 'chats':       return `${nav}<app-chats></app-chats>`;
            case 'login':       return `<app-auth></app-auth>`;
            case 'profile':     return `${nav}<app-profile></app-profile>`;
            case 'terminal':    return `${nav}<app-terminal></app-terminal>`;
            case 'post-create': return `${nav}<app-post-create></app-post-create>`;
            case 'settings': return `${nav}<app-settings></app-settings>`;
            case 'post-page': return `${nav}<app-post-page></app-post-page>`;
            default:            return nav;
        }
    },

    // ----- navigation -----

    initNavigation(): void {
        window.addEventListener('app-navigate', (e: Event) => {
            const detail    = (e as CustomEvent).detail as { page: string; postId?: number };
            const targetPage = detail.page as PageType;
            if (targetPage === 'settings' && !this.state.user) return;
            if (targetPage) {
                this.state.lastpage = this.state.page;
                this.state.page     = targetPage;
                if (detail.postId != null) this.state.postId = detail.postId;
                history.pushState({ page: targetPage, postId: detail.postId }, '', `/#${targetPage}`);
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
                    history.pushState({ page: targetPage }, '', `/#${targetPage}`);
                    this.render();
                }
            }
        });

        window.addEventListener('popstate', (e: PopStateEvent) => {
            this.state.page   = e.state?.page   || 'feeds';
            this.state.postId = e.state?.postId ?? undefined;
            this.render();
        });
    },
};

App.init();
