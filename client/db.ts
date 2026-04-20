import { LOGO } from "./assets";

interface Msg {
    id: string;
    text: string;
    files: string;
    sender: string;
    timestamp: number;
}

interface Chat {
    id: string,
    name: string,
    avatar: string,
}

type MsgPool = Msg[];
type ChatsPool = Chat[];


type CacheEntry = string | MsgPool | ChatsPool;


export class HntDataBase {
    private storage = new Map<string, CacheEntry>();

    private readonly DEFAULT_MSG_TTL = 1000 * 60 * 30;
    private readonly MAX_MESSAGES_PER_CHANNEL = 50; // Лимит обьектов в бд
    private readonly MAX_CACHED_CHATS = 50;

    public get_chats(): ChatsPool | undefined {
        return this.storage.get(`chat_pool`) as ChatsPool;
    }

    public get_msg_pool(chatid: string): MsgPool | undefined {
        return this.storage.get(`chat:${chatid}`) as MsgPool;
    }

    /**
     * Добавляет новое сообщение в пул конкретного чата.
     * Если пула нет — создает его. Если лимит превышен — удаляет старое.
     */
    public add_msg(chatid: string, msg: Msg): void {
        const key = `chat:${chatid}`;
        let pool = this.get_msg_pool(chatid) || [];

        // Добавляем новое сообщение в конец
        pool.push(msg);

        // Если сообщений больше лимита, удаляем самое старое (первое)
        if (pool.length > this.MAX_MESSAGES_PER_CHANNEL) {
            pool.shift();
        }

        this.storage.set(key, pool);
    }

    /**
     * Добавляет новый чат в общий список чатов.
     * Проверяет на дубликаты по id и следит за общим лимитом чатов.
     */
    public add_chat(chat: Chat): void {
        const key = `chat_pool`;
        let chats = this.get_chats() || [];

        // Проверяем, нет ли уже такого чата в списке (чтобы не дублировать)
        const exists = chats.some(c => c.id === chat.id);
        if (!exists) {
            chats.push(chat);
        } else {
            // Опционально: обновляем данные чата, если он уже есть
            chats = chats.map(c => c.id === chat.id ? chat : c);
        }

        // Если чатов слишком много, удаляем самый старый из кэша
        if (chats.length > this.MAX_CACHED_CHATS) {
            chats.shift();
        }

        this.storage.set(key, chats);
    }

    public get_value(key: string): string | undefined {
        return this.storage.get(key) as string;
    }

    public del(channelId: string): void {
        this.storage.delete(channelId);
    }

}

/**
 * Инициализирует базу данных и наполняет её тестовыми данными.
 * Аватарки и файлы генерируются как Base64 заглушки.
 */
export function init_test_db(): HntDataBase {
    const db = new HntDataBase();

    const mockBase64Avatar = LOGO;

    // Заглушки Base64 для аватарок (прозрачные пиксели разных цветов или иконки)
    const mockBase64File = "data:application/pdf;base64,JVBERi0xLjcKCjEgMCBvYmoKPDwvVHlwZSAvQ2F0YWxvZyAvUGFnZXMgMiAwIFI+PgplbmRvYmoKMiAwIG9iago8PC9UeXBlIC9QYWdlcyAvS2lkcyBbMyAwIFJdIC9Db3VudCAxPj4KZW5kb2JqCjMgMCBvYmoKPDwvVHlwZSAvUGFnZSAvUGFyZW50IDIgMCBSIC9NZWRpYUJveCBbMCAwIDYxMiA3OTJdPj4KZW5kb2JqCnhyZWYKMCA0CjAwMDAwMDAwMDAgNjU1MzUgZiAKMDAwMDAwMDAxOCAwMDAwMCBuIAowMDAwMDAwMDc3IDAwMDAwIG4gCjAwMDAwMDAxNTYgMDAwMDAgbiAKdHJhaWxlcgo8PC9TaXplIDQvUm9vdCAxIDAgUj4+CnN0YXJ0eHJlZgoyNTIKJSVFT0YK";

    // 1. Создаем список тестовых чатов
    const testChats: Chat[] = [
        { id: "1", name: "Общий чат", avatar: mockBase64Avatar },
        { id: "2", name: "Разработка", avatar: mockBase64Avatar },
        { id: "3", name: "Дизайн", avatar: mockBase64Avatar }
    ];

    testChats.forEach(chat => db.add_chat(chat));

    // 2. Наполняем чаты сообщениями
    const users = ["Alice", "Bob", "Charlie"];

    testChats.forEach(chat => {
        // Генерируем по 10 сообщений
        for (let i = 0; i < 10; i++) {
            const hasFile = i % 4 === 0; // Каждое 4-е сообщение будет с "файлом"

            const msg: Msg = {
                id: `m_${chat.id}_${i}`,
                sender: users[Math.floor(Math.random() * users.length)],
                      text: `Привет! Это сообщение #${i + 1}`,
                      // Если есть файл, кладем base64 строку, если нет — пустая строка
                      files: hasFile ? mockBase64File : "",
                      timestamp: Date.now() - (10 - i) * 100000
            };

            db.add_msg(chat.id, msg);
        }
    });

    console.log("Тестовая БД готова (Base64 Mode)");
    return db;
}

