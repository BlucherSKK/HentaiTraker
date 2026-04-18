import { HOME_BODY, LOGO, STYLES } from "./assets";
import { get_header } from "./header";
import { Feed, HomeNav } from "./home";
import { AppNav } from "./nav";



enum Tags {
    Any,
    Hentai
}

export interface User {
    name: string;
    token: string;
    tagpool: Tags[];
}


let RecentPosts: string = "";

// 0. Интерфейсы для типизации
interface AppState {
    page: 'feeds' | 'projects' | 'settings' | 'login' | 'dm' | "chats";
    lastpage: 'feeds' | 'projects' | 'settings' | 'login' | 'dm' | 'chats';
    user?: User;
    items: string[];
    init: boolean;
}

customElements.define('app-feed', Feed);
customElements.define('home-nav', HomeNav);
customElements.define('app-nav', AppNav);

const App = {
    // 1. Состояние приложения с явным типом
    state: {
        page: 'feeds',
        lastpage: 'feeds',
        items: ['Разработка на Rust', 'Настройка Arch Linux', 'Docker контейнеры'],
        init: false,
    } as AppState,

    // 2. Инициализация
    init(): void {
        console.log("SPA Приложение запущено!");
        this.render();
        this.initNavigation();
    },

    // 3. Главный движок отрисовки
    render(): void {
        if (this.state.init === false) {
            this.state.page = 'feeds';
            this.state.init = true;
            const root = document.getElementById('app');
            if (!root) {
                console.error("Элемент #app не найден");
                return;
            }

            let content: string = '';

            content = `
                ${get_header("home", this.state.user)}
                <app-nav data-link="${this.state.page}"></app-nav>
                <hero id="apphero">
                    <app-feed />
                </hero>
                `
            root.innerHTML = content;
        } else {
            const root = document.getElementById('apphero');
            if (!root) {
                console.error("Элемент #apphero не найден");
                return;
            }

            let content: string = '';

            switch (this.state.page) {
                case 'feeds':
                    content = `
                    <app-feed />
                    `
                    break;
                default:
                    content = ""
                    break;
            }
            root.innerHTML = content;

        }

        this.applyStyles();
    },

    // 4. Навигация через History API
    initNavigation(): void {
        window.addEventListener('click', (e: MouseEvent) => {
            const target = e.target as HTMLElement;
            const link = target.closest<HTMLElement>('[data-link]');

            if (link) {
                e.preventDefault();
                const targetPage = link.getAttribute('data-link') as AppState['page'];

                if (targetPage) {
                    this.state.lastpage = this.state.page;
                    this.state.page = targetPage;
                    // Исправляем типизацию History API
                    history.pushState({ page: targetPage }, "", `/${targetPage}`);
                    this.render();
                }
            }
        });

        window.addEventListener('popstate', (e: PopStateEvent) => {
            // e.state — это тот объект { page: targetPage }, который мы пушили
            if (e.state && e.state.page) {
                this.state.page = e.state.page;
                this.render();
            } else {
                // Если стейта нет (например, вернулись в самый низ истории), ставим home
                this.state.page = 'feeds';
                this.render();
            }
        });
    },

    // 5. Динамические стили
    applyStyles(): void {
        if (document.getElementById('app-styles')) return;

        const style = document.createElement('style');
        style.id = 'app-styles';
        style.textContent = STYLES;
        document.head.appendChild(style);
    }
};

// Запуск
App.init();
