import { HOME_BODY, LOGO, STYLES } from "./assets";
import { get_header } from "./header";
import { Feed, HomeNav } from "./home";



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
    page: 'home' | 'projects' | 'settings' | 'login';
    lastpage: 'home' | 'projects' | 'settings' | 'login';
    user?: User;
    items: string[];
}

customElements.define('app-feed', Feed);
customElements.define('home-nav', HomeNav);

const App = {
    // 1. Состояние приложения с явным типом
    state: {
        page: 'home',
        lastpage: 'home',
        items: ['Разработка на Rust', 'Настройка Arch Linux', 'Docker контейнеры']
    } as AppState,

    // 2. Инициализация
    init(): void {
        console.log("SPA Приложение запущено!");
        this.render();
        this.initNavigation();
    },

    // 3. Главный движок отрисовки
    render(): void {
        const root = document.getElementById('app');
        if (!root) {
            console.error("Элемент #app не найден");
            return;
        }

        let content: string = '';

        if (this.state.page === 'home') {

            content = `
            ${get_header("home", this.state.user)}
            <app-feed />
            ${HOME_BODY}
            `;
        } else if (this.state.page === 'projects') {
            content = `
            <div class="container">
            <h1>Мои проекты</h1>
            <ul>
            ${this.state.items.map(item => `<li>${item}</li>`).join('')}
            </ul>
            <button class="btn-back" data-link="home">← Назад</button>
            </div>
            `;
        } else if (this.state.page === 'settings') {
            content = `
            <div class="container">
            <h1>Настройки сервера</h1>
            <p>Здесь будут параметры конфигурации...</p>
            <button class="btn-back" data-link="home">← Назад</button>
            </div>
            `;
        } else if (this.state.pahe === 'login') {
            content = `
            <a>fuck u</a>
            `
        }

        root.innerHTML = content;
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
                this.state.page = 'home';
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
