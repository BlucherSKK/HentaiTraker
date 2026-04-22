use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub mod postgress;
pub mod redis;

use postgress::Database;
use redis::{RedisClient, RedisInitError};

// ─── Domain types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub enum Tags { Hentai, Any }

#[derive(Debug, Serialize, Deserialize)]
pub enum AttachmentType { Torrent, Png, Gif, Jpg }

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub pass: String,
    pub last_visit: NaiveDateTime,
    pub roles: Option<String>,
    pub avatar: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Post {
    pub id: i32,
    pub title: Option<String>,
    pub content: String,
    pub files: Option<String>,
    pub author_id: i32,
    pub time: NaiveDateTime,
    pub tags: Option<String>,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Chat {
    pub id: i32,
    pub title: Option<String>,
    pub content: String,
    pub images: Option<String>,
    pub author_id: i32,
    pub time: NaiveDateTime,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Message {
    pub id: i32,
    pub content: String,
    pub files: Option<String>,
    pub author_id: i32,
    pub chat_id: i32,
    pub time: NaiveDateTime,
}

// ─── Post builder ────────────────────────────────────────────────────────────

const SERIALIZE_TAGS_RESOLUTION: u8 = 3;

impl Post {
    pub fn new(author_id: i32, title: Option<String>, content: String, tags: Vec<Tags>) -> Self {
        debug!("constructing post with title {:?}", title);
        Self {
            id: -1, title, content, files: None, author_id,
            time: Utc::now().naive_utc(),
            tags: Self::encode_tags(&tags),
        }
    }

    pub fn encode_tags(tags: &[Tags]) -> Option<String> {
        if tags.is_empty() { return None; }
        let body: String = tags.iter().map(|t| match t {
            Tags::Hentai => "hnt",
            Tags::Any    => "any",
        }).collect();
        Some(format!("{}{}", SERIALIZE_TAGS_RESOLUTION, body))
    }
}

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum StoreError {
    Db(sqlx::Error),
    Redis(RedisInitError),
    Json(serde_json::Error),
    /// Запрашиваемая запись не найдена.
    NotFound,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Db(e)    => write!(f, "db: {e}"),
            StoreError::Redis(e) => write!(f, "redis init: {e}"),
            StoreError::Json(e)  => write!(f, "json: {e}"),
            StoreError::NotFound => write!(f, "not found"),
        }
    }
}

impl std::error::Error for StoreError {}
impl From<sqlx::Error>       for StoreError { fn from(e: sqlx::Error)       -> Self { StoreError::Db(e) } }
impl From<serde_json::Error> for StoreError { fn from(e: serde_json::Error) -> Self { StoreError::Json(e) } }

// ─── Store ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Store {
    db:    Database,
    cache: RedisClient,
}

const CACHE_THRESHOLD: u64 = 5;
const COUNTER_TTL:     u64 = 3_600;

impl Store {
    pub async fn init(db_url: &str, redis_url: &str) -> Result<Self, StoreError> {
        let db    = Database::init(db_url).await.map_err(StoreError::Db)?;
        let cache = RedisClient::init(redis_url).map_err(StoreError::Redis)?;
        Ok(Self { db, cache })
    }

    async fn cache_get<T: for<'de> Deserialize<'de>>(cache: &RedisClient, key: &str) -> Option<T> {
        serde_json::from_str(&cache.get(key).await?).ok()
    }

    async fn cache_set<T: Serialize>(cache: &RedisClient, key: &str, val: &T, ttl: u64) {
        if let Ok(json) = serde_json::to_string(val) {
            cache.set_ex(key, &json, ttl).await;
        }
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    // В src/db/mod.rs, внутри impl Store:
    pub async fn db_is_member(&self, chat_id: i32, member_id: i32) -> Result<bool, StoreError> {
        Ok(self.db.is_chat_member(chat_id, member_id).await?)
    }


    pub async fn get_user(&self, id: i32) -> Result<Option<User>, StoreError> {
        let ck = format!("user:{id}");
        if let Some(u) = Self::cache_get::<User>(&self.cache, &ck).await { return Ok(Some(u)); }
        let hits = self.cache.incr_counter(&format!("access:user:{id}"), COUNTER_TTL).await;
        let user = self.db.get_user_by_id(id).await?;
        if hits >= CACHE_THRESHOLD {
            if let Some(ref u) = user { Self::cache_set(&self.cache, &ck, u, 300).await; }
        }
        Ok(user)
    }

    /// Поиск по имени — используется при логине. Намеренно без кеша.
    pub async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, StoreError> {
        Ok(self.db.get_user_by_name(name).await?)
    }

    pub async fn set_user(&self, name: &str, pass: &str, roles: &str) -> Result<User, StoreError> {
        let user = self.db.insert_user(name, pass, roles).await?;
        self.cache.del(&format!("user:{}", user.id)).await;
        Ok(user)
    }

    // ── Posts ─────────────────────────────────────────────────────────────────

    pub async fn get_posts_by_author(&self, author_id: i32, limit: i64) -> Result<Vec<Post>, StoreError> {
        let ck = format!("posts:author:{author_id}:lim:{limit}");
        if let Some(p) = Self::cache_get::<Vec<Post>>(&self.cache, &ck).await { return Ok(p); }
        let hits = self.cache.incr_counter(&format!("access:posts:author:{author_id}"), COUNTER_TTL).await;
        let posts = self.db.get_posts_by_author(author_id, limit).await?;
        if hits >= CACHE_THRESHOLD { Self::cache_set(&self.cache, &ck, &posts, 300).await; }
        Ok(posts)
    }

    pub async fn get_latest_post_before(&self, time: NaiveDateTime) -> Result<Option<Post>, StoreError> {
        Ok(self.db.get_latest_post_before(time).await?)
    }

    pub async fn get_latest_post_now(&self) -> Result<Option<Post>, StoreError> {
        const CK: &str = "feed:latest";
        if let Some(p) = Self::cache_get::<Post>(&self.cache, CK).await { return Ok(Some(p)); }
        let hits = self.cache.incr_counter("access:feed:latest", COUNTER_TTL).await;
        let post = self.db.get_latest_post_now().await?;
        if hits >= CACHE_THRESHOLD {
            if let Some(ref p) = post { Self::cache_set(&self.cache, CK, p, 30).await; }
        }
        Ok(post)
    }

    /// Последние N постов для REST /api/getfeed.
    pub async fn get_latest_posts(&self, limit: i64) -> Result<Vec<Post>, StoreError> {
        let ck = format!("feed:posts:{limit}");
        if let Some(p) = Self::cache_get::<Vec<Post>>(&self.cache, &ck).await { return Ok(p); }
        let hits = self.cache.incr_counter(&format!("access:feed:posts:{limit}"), COUNTER_TTL).await;
        let posts = self.db.get_latest_posts(limit).await?;
        if hits >= CACHE_THRESHOLD { Self::cache_set(&self.cache, &ck, &posts, 30).await; }
        Ok(posts)
    }

    pub async fn add_post(&self, author_id: i32, title: Option<&str>, content: &str) -> Result<Post, StoreError> {
        let post = self.db.insert_post(author_id, title, content).await?;
        for lim in [20i64, 50, 100] {
            self.cache.del(&format!("posts:author:{author_id}:lim:{lim}")).await;
            self.cache.del(&format!("feed:posts:{lim}")).await;
        }
        self.cache.del("feed:latest").await;
        Ok(post)
    }

    // ── Chats ─────────────────────────────────────────────────────────────────

    pub async fn get_chat_by_id(&self, chat_id: i32) -> Result<Option<Chat>, StoreError> {
        let ck = format!("chat:{chat_id}");
        if let Some(c) = Self::cache_get::<Chat>(&self.cache, &ck).await { return Ok(Some(c)); }
        let hits = self.cache.incr_counter(&format!("access:chat:{chat_id}"), COUNTER_TTL).await;
        let chat = self.db.get_chat_by_id(chat_id).await?;
        if hits >= CACHE_THRESHOLD {
            if let Some(ref c) = chat { Self::cache_set(&self.cache, &ck, c, 600).await; }
        }
        Ok(chat)
    }

    /// Все чаты, в которых состоит пользователь.
    pub async fn get_user_chats(&self, member_id: i32) -> Result<Vec<Chat>, StoreError> {
        let ck = format!("user:chats:{member_id}");
        if let Some(c) = Self::cache_get::<Vec<Chat>>(&self.cache, &ck).await { return Ok(c); }
        let hits = self.cache.incr_counter(&format!("access:user:chats:{member_id}"), COUNTER_TTL).await;
        let chats = self.db.get_user_chats(member_id).await?;
        if hits >= CACHE_THRESHOLD { Self::cache_set(&self.cache, &ck, &chats, 120).await; }
        Ok(chats)
    }

    pub async fn create_chat(&self, author_id: i32, title: Option<&str>, content: &str) -> Result<Chat, StoreError> {
        Ok(self.db.create_chat(author_id, title, content).await?)
    }

    /// Добавить участника в чат (idempotent).
    /// Инвалидирует кеш списка чатов пользователя.
    pub async fn add_chat_member(&self, chat_id: i32, member_id: i32) -> Result<(), StoreError> {
        self.db.add_chat_member(chat_id, member_id).await?;
        self.cache.del(&format!("user:chats:{member_id}")).await;
        Ok(())
    }

    /// Вступление пользователя в чат: проверяет существование чата,
    /// добавляет в cross_chat_members, инвалидирует кеш.
    /// Возвращает данные чата (нужны клиенту после подтверждения).
    /// Ошибка `StoreError::NotFound` — чат не существует.
    pub async fn join_chat(&self, chat_id: i32, member_id: i32) -> Result<Chat, StoreError> {
        let chat = self.db.get_chat_by_id(chat_id).await?
        .ok_or(StoreError::NotFound)?;
        // ON CONFLICT DO NOTHING делает операцию idempotent
        self.db.add_chat_member(chat_id, member_id).await?;
        self.cache.del(&format!("user:chats:{member_id}")).await;
        Ok(chat)
    }

    // ── Messages ──────────────────────────────────────────────────────────────

    pub async fn get_chat_messages(&self, chat_id: i32, limit: i64) -> Result<Vec<Message>, StoreError> {
        let ck = format!("chat:msgs:{chat_id}:lim:{limit}");
        if let Some(m) = Self::cache_get::<Vec<Message>>(&self.cache, &ck).await { return Ok(m); }
        let hits = self.cache.incr_counter(&format!("access:chat:msgs:{chat_id}"), COUNTER_TTL).await;
        let msgs = self.db.get_chat_messages(chat_id, limit).await?;
        if hits >= CACHE_THRESHOLD { Self::cache_set(&self.cache, &ck, &msgs, 60).await; }
        Ok(msgs)
    }

    pub async fn send_message(&self, chat_id: i32, author_id: i32, content: &str) -> Result<Message, StoreError> {
        let msg = self.db.send_message(chat_id, author_id, content).await?;
        for lim in [20i64, 50, 100] {
            self.cache.del(&format!("chat:msgs:{chat_id}:lim:{lim}")).await;
        }
        Ok(msg)
    }
}
