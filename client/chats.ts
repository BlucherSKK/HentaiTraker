import { HntDataBase } from "./db";


export class Chats extends HTMLElement {


    public db!: HntDataBase;

    static get observedAttributes() {
        return ['data-selected-id'];
    }

    attributeChangedCallback(name: string, oldValue: string, newValue: string) {
        if (oldValue !== newValue) {
            this.render();
        }
    }

    connectedCallback() {
        this.render();
    }

    render() {
        const selectedChatId = this.getAttribute('data-selected-id');

        this.innerHTML = `
        <div class="chat-container">
            <aside class="chat-sidebar">
                <div class="chat-header">Чаты</div>
                    <div style="overflow-y: auto;">
                    ${this.renderChatList()}
                </div>
            </aside>

            <main class="chat-main">
                ${selectedChatId ? this.renderActiveChat(selectedChatId) : this.renderEmpty()}
            </main>

            <aside class="chat-right-panel">
                <div class="chat-header">Участники</div>
                    <div style="padding: 10px;">
                    ${selectedChatId ? this.renderMemberBar(selectedChatId) : ''}
                </div>
            </aside>
        </div>
        `;
    }

    private renderEmpty() {
        return `<div class="chat-empty-state">Выберите чат для начала общения</div>`;
    }

    private renderActiveChat(id: string) {
        return `
        <div class="chat-header">Чат # ${id}</div>
        <div class="message-list">
        <div style="color: #ccc">Загрузка истории...</div>
        </div>
        <div class="chat-input-area">
        <input type="text" placeholder="Введите сообщение...">
        </div>
        `;
    }

    private renderChatList(): string {

        if (!this.db) {
            return `<div class="list-item">Ошибка инициализации БД</div>`;
        }

        const chats = this.db.get_chats();

        // 2. Если чатов еще нет или они грузятся
        if (!chats || chats.length === 0) {
            return `<div class="list-item">Чатов пока нет...</div>`;
        }

        // 3. Рендерим список, "топая" от полученных данных
        return chats.map(chat => `
        <div class="list-item ${this.getAttribute('data-selected-id') === chat.id ? 'active' : ''}"
        onclick="this.closest('app-chats').setAttribute('data-selected-id', '${chat.id}')">
        <img src="${chat.avatar || 'default-avatar.png'}" class="chat-mini-avatar" alt="avatar">
        <div class="chat-info">
        <div class="chat-name">${chat.name}</div>
        <div class="chat-last-msg">Нажмите, чтобы открыть</div>
        </div>
        </div>
        `).join('');
    }

    private renderMemberBar(id: string): string {
        // Заглушка отрисовки участников
        return `<div style="font-size: 0.9em; color: #666;">Загрузка списка...</div>`;
    }

    // --- ПРОТОТИПЫ ГЕТТЕРОВ (для будущей реализации) ---

}

