interface Msg {
    id: string;
    text: string;
    files: string[];
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
    private readonly MAX_MESSAGES_PER_CHANNEL = 50;
    private readonly MAX_CACHED_CHATS = 50;

    public get_chats(): ChatsPool | undefined {
        return this.storage.get(`chat_pool`) as ChatsPool;
    }

    public get_msg_pool(chatid: string): MsgPool | undefined {
        return this.storage.get(`chat:${chatid}`) as MsgPool;
    }

    public add_msg(chatid: string, msg: Msg): void {
        const key = `chat:${chatid}`;
        let pool = this.get_msg_pool(chatid) || [];

        pool.push(msg);

        if (pool.length > this.MAX_MESSAGES_PER_CHANNEL) {
            pool.shift();
        }

        this.storage.set(key, pool);
    }

    public add_chat(chat: Chat): void {
        const key = `chat_pool`;
        let chats = this.get_chats() || [];

        const exists = chats.some(c => c.id === chat.id);
        if (!exists) {
            chats.push(chat);
        } else {
            chats = chats.map(c => c.id === chat.id ? chat : c);
        }

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

export function init_test_db(): HntDataBase {
    const db = new HntDataBase();

    const testChats: Chat[] = [
        { id: "1", name: "Общий чат",  avatar: "" },
        { id: "2", name: "Разработка", avatar: "" },
        { id: "3", name: "Дизайн",     avatar: "" },
    ];

    testChats.forEach(chat => db.add_chat(chat));

    const users = ["Alice", "Bob", "Charlie"];

    testChats.forEach(chat => {
        for (let i = 0; i < 10; i++) {
            const msg: Msg = {
                id:        `m_${chat.id}_${i}`,
                sender:    users[Math.floor(Math.random() * users.length)],
                      text:      `Привет! Это сообщение #${i + 1}`,
                      files:     [],
                      timestamp: Date.now() - (10 - i) * 100000,
            };
            db.add_msg(chat.id, msg);
        }
    });

    return db;
}
