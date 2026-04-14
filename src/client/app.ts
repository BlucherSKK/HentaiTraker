import { HOME_BODY, LOGO, STYLES } from "./assets";
import { get_header } from "./header";
import { define_homenav, HomeNav } from "./home";


export interface User {
    name: string;
    token: string;
}
// 0. Интерфейсы для типизации
interface AppState {
    page: 'home' | 'projects' | 'settings';
    user?: User;
    items: string[];
}

define_homenav();

const App = {
    // 1. Состояние приложения с явным типом
    state: {
        page: 'home',
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
                    this.state.page = targetPage;
                    // Исправляем типизацию History API
                    history.pushState({ page: targetPage }, "", `/${targetPage}`);
                    this.render();
                }
            }
        });

        window.addEventListener('popstate', (e: PopStateEvent) => {
            if (e.state && (e.state as AppState).page) {
                this.state.page = (e.state as AppState).page;
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
