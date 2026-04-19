interface ChatMessage {
    id: string;
    text: string;
    sender: string;
    timestamp: number;
}

type CacheEntry = Map<string, any> | ChatMessage;

export class HntDataBase {
    // Основное хранилище: Ключ - ID канала, Значение - объект с сообщениями и TTL
    private storage = new Map<string, CacheEntry>();

    private readonly DEFAULT_TTL = 1000 * 60 * 30; // 30 минут
    private readonly MAX_MESSAGES_PER_CHANNEL = 50; // Лимит как в LRU-кэше

    public set(channelId: string, message: ChatMessage): void {
        const current = this.storage.get(channelId) || {
            messages: [],
            expiresAt: Date.now() + this.DEFAULT_TTL
        };

        // Добавляем новое сообщение
        current.messages.push(message);

        // Ограничиваем размер (чтобы вкладка не "съела" всю RAM)
        if (current.messages.length > this.MAX_MESSAGES_PER_CHANNEL) {
            current.messages.shift(); // Удаляем старое (как LTRIM в Redis)
        }

        // Обновляем TTL при активности
        current.expiresAt = Date.now() + this.DEFAULT_TTL;

        this.storage.set(channelId, current);
    }

    /**
     * Получить сообщения канала (аналог LRANGE)
     */
    public get(channelId: string): ChatMessage[] {
        const entry = this.storage.get(channelId);

        if (!entry) return [];

        // Проверка TTL (expired?)
        if (Date.now() > entry.expiresAt) {
            this.storage.delete(channelId);
            return [];
        }

        return entry.messages;
    }

    /**
     * Удалить кэш канала (аналог DEL)
     */
    public del(channelId: string): void {
        this.storage.delete(channelId);
    }

    /**
     * Очистить старые данные (запускается по таймеру)
     */
    public flushExpired(): void {
        const now = Date.now();
        for (const [key, value] of this.storage.entries()) {
            if (now > value.expiresAt) {
                this.storage.delete(key);
            }
        }
    }
}

// Экспортируем как синглтон для всего приложения
export const messageCache = new MessageRedisClone();
