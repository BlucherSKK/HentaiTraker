

export class Chats extends HTMLElement {
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
                <div class="chat-header">Диалоги</div>
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
                    ${selectedChatId ? this.renderParticipants(selectedChatId) : ''}
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
        // Заглушка отрисовки списка
        return `<div class="list-item">Загрузка чатов...</div>`;
    }

    private renderParticipants(id: string): string {
        // Заглушка отрисовки участников
        return `<div style="font-size: 0.9em; color: #666;">Загрузка списка...</div>`;
    }

    // --- ПРОТОТИПЫ ГЕТТЕРОВ (для будущей реализации) ---

    /** Возвращает массив текущих чатов */
    get chatsList(): any[] {
        return [];
    }

    /** Возвращает сообщения активного чата из кэша или стейта */
    get activeMessages(): any[] {
        return [];
    }

    /** Возвращает список участников для выбранного чата */
    get currentParticipants(): any[] {
        return [];
    }

    /** Геттер для получения мета-данных (например, онлайн ли собеседник) */
    get chatMetadata(): object {
        return {};
    }
}

