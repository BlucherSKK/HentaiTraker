import { STYLES } from "./assets";
import { Chats } from "./chats";
import { get_header, get_nonlogin_dm_noty } from "./header";
import { Feed, HomeNav } from "./home";
import { AppNav } from "./nav";

enum Tags {
    Any,
    Hentai
}

export interface User {
    name: string;
    id: string;
    token: string;
    tagpool: Tags[];
}

type PageType = 'feeds' | 'projects' | 'settings' | 'login' | 'dm' | 'chats';

interface AppState {
    page: PageType;
    lastpage: PageType;
    user?: User;
    items: string[];
    init: boolean;
}

customElements.define('app-feed', Feed);
customElements.define('home-nav', HomeNav);
customElements.define('app-nav', AppNav);
customElements.define('app-chats', Chats);

const App = {
    state: {
        page: 'feeds',
        lastpage: 'feeds',
        items: ['Разработка на Rust', 'Настройка Arch Linux', 'Docker контейнеры'],
        init: false,
    } as AppState,

    init(): void {
        this.applyStyles();
        this.render();
        this.initNavigation();
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

        hero.innerHTML = this.getContentByPage();
    },

    getContentByPage(): string {
        const nav = `<app-nav data-link="${this.state.page}"></app-nav>`;

        switch (this.state.page) {
            case 'feeds':
                return `${nav}<app-feed></app-feed>`;
            case 'dm':
                return `${nav}${this.state.user ? "" : get_nonlogin_dm_noty()}`;
            case 'chats':
                return `${nav}<app-chats />`;
            default:
                return nav;
        }
    },

    initNavigation(): void {
        window.addEventListener('click', (e: MouseEvent) => {
            const target = e.target as HTMLElement;
            const link = target.closest<HTMLElement>('[data-link]');

            if (link) {
                e.preventDefault();
                const targetPage = link.getAttribute('data-link') as PageType;

                if (targetPage && targetPage !== this.state.page) {
                    this.state.lastpage = this.state.page;
                    this.state.page = targetPage;
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

    applyStyles(): void {
        if (document.getElementById('app-styles')) return;

        const style = document.createElement('style');
        style.id = 'app-styles';
        style.textContent = STYLES;
        document.head.appendChild(style);
    }
};

App.init();
