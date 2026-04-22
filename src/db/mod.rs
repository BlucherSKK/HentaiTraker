use chrono::{NaiveDateTime, Utc};
use log::debug;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub mod postgress;
pub mod redis;

use postgress::Database;
use redis::{RedisClient, RedisInitError};

// ─── Domain types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub enum Tags {
    Hentai,
    Any,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AttachmentType {
    Torrent,
    Png,
    Gif,
    Jpg,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub pass: String,
    pub last_visit: NaiveDateTime,
    pub roles: Option<String>,
    pub avatar: Option<String>,
    /// Serialized tag string stored in DB (e.g. "3hnthnt"). Use Tag helpers to parse.
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
    /// Serialized tag string (same format as User.tags).
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
    pub fn new(
        author_id: i32,
        title: Option<String>,
        content: String,
        tags: Vec<Tags>,
    ) -> Self {
        debug!("constructing post with title {:?}", title);
        Self {
            id: -1,
            title,
            content,
            files: None,
            author_id,
            time: Utc::now().naive_utc(),
            tags: Self::encode_tags(&tags),
        }
    }

    pub fn encode_tags(tags: &[Tags]) -> Option<String> {
        if tags.is_empty() {
            return None;
        }
        let body: String = tags
        .iter()
        .map(|t| match t {
            Tags::Hentai => "hnt",
            Tags::Any => "any",
        })
        .collect();
        Some(format!("{}{}", SERIALIZE_TAGS_RESOLUTION, body))
    }
}

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum StoreError {
    Db(sqlx::Error),
    Redis(RedisInitError),
    Json(serde_json::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Db(e) => write!(f, "db: {e}"),
            StoreError::Redis(e) => write!(f, "redis init: {e}"),
            StoreError::Json(e) => write!(f, "json: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<sqlx::Error> for StoreError {
    fn from(e: sqlx::Error) -> Self {
        StoreError::Db(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Json(e)
    }
}

// ─── Store ───────────────────────────────────────────────────────────────────

/// Public façade over PostgreSQL + Redis.
///
/// Getter flow:
///   1. Check Redis cache → return on hit.
///   2. Increment access counter for this key.
///   3. Fetch from PostgreSQL.
///   4. If counter ≥ CACHE_THRESHOLD → write to Redis with TTL.
///
/// Setter flow:
///   1. Write to PostgreSQL.
///   2. Invalidate the corresponding Redis key so the next getter
///      gets fresh data.
#[derive(Clone)]
pub struct Store {
    db: Database,
    cache: RedisClient,
}

/// Cache data in Redis after this many DB hits for the same key.
const CACHE_THRESHOLD: u64 = 5;
/// Access counters expire after 1 hour of inactivity (seconds).
const COUNTER_TTL: u64 = 3_600;

impl Store {
    pub async fn init(db_url: &str, redis_url: &str) -> Result<Self, StoreError> {
        let db = Database::init(db_url).await.map_err(StoreError::Db)?;
        let cache = RedisClient::init(redis_url).map_err(StoreError::Redis)?;
        Ok(Self { db, cache })
    }

    // ── internal helpers ────────────────────────────────────────────────────

    async fn cache_get<T: for<'de> Deserialize<'de>>(
        cache: &RedisClient,
        key: &str,
    ) -> Option<T> {
        let json = cache.get(key).await?;
        serde_json::from_str(&json).ok()
    }

    async fn cache_set<T: Serialize>(cache: &RedisClient, key: &str, value: &T, ttl: u64) {
        if let Ok(json) = serde_json::to_string(value) {
            cache.set_ex(key, &json, ttl).await;
        }
    }

    // ── USER GETTERS / SETTERS ───────────────────────────────────────────────

    /// Getter: Redis → PostgreSQL, caches after CACHE_THRESHOLD hits.
    pub async fn get_user(&self, id: i32) -> Result<Option<User>, StoreError> {
        let ck = format!("user:{id}");

        if let Some(user) = Self::cache_get::<User>(&self.cache, &ck).await {
            return Ok(Some(user));
        }

        let hits = self.cache.incr_counter(&format!("access:user:{id}"), COUNTER_TTL).await;
        let user = self.db.get_user_by_id(id).await?;

        if hits >= CACHE_THRESHOLD {
            if let Some(ref u) = user {
                Self::cache_set(&self.cache, &ck, u, 300).await;
            }
        }
        Ok(user)
    }

    /// Setter: writes to PostgreSQL, invalidates Redis cache.
    pub async fn set_user(
        &self,
        name: &str,
        pass: &str,
        roles: &str,
    ) -> Result<User, StoreError> {
        let user = self.db.insert_user(name, pass, roles).await?;
        self.cache.del(&format!("user:{}", user.id)).await;
        Ok(user)
    }

    // ── POST GETTERS / SETTERS ───────────────────────────────────────────────

    /// Getter: returns paginated posts for an author.
    /// Cache key includes limit so different page sizes don't collide.
    pub async fn get_posts_by_author(
        &self,
        author_id: i32,
        limit: i64,
    ) -> Result<Vec<Post>, StoreError> {
        let ck = format!("posts:author:{author_id}:lim:{limit}");

        if let Some(posts) = Self::cache_get::<Vec<Post>>(&self.cache, &ck).await {
            return Ok(posts);
        }

        let hits = self
        .cache
        .incr_counter(&format!("access:posts:author:{author_id}"), COUNTER_TTL)
        .await;
        let posts = self.db.get_posts_by_author(author_id, limit).await?;

        if hits >= CACHE_THRESHOLD {
            Self::cache_set(&self.cache, &ck, &posts, 300).await;
        }
        Ok(posts)
    }

    /// Getter: latest post strictly before `time`. Not cached — argument is
    /// highly variable (callers pass their current cursor timestamp).
    pub async fn get_latest_post_before(
        &self,
        time: NaiveDateTime,
    ) -> Result<Option<Post>, StoreError> {
        Ok(self.db.get_latest_post_before(time).await?)
    }

    /// Getter: latest post before NOW(). Short TTL cache — used for feed polling.
    pub async fn get_latest_post_now(&self) -> Result<Option<Post>, StoreError> {
        const CK: &str = "feed:latest";

        if let Some(post) = Self::cache_get::<Post>(&self.cache, CK).await {
            return Ok(Some(post));
        }

        let hits = self.cache.incr_counter("access:feed:latest", COUNTER_TTL).await;
        let post = self.db.get_latest_post_now().await?;

        if hits >= CACHE_THRESHOLD {
            if let Some(ref p) = post {
                Self::cache_set(&self.cache, CK, p, 30).await;
            }
        }
        Ok(post)
    }

    /// Setter: insert post, invalidate author's post-list cache.
    pub async fn add_post(
        &self,
        author_id: i32,
        title: Option<&str>,
        content: &str,
    ) -> Result<Post, StoreError> {
        let post = self.db.insert_post(author_id, title, content).await?;
        for lim in [20i64, 50, 100] {
            self.cache
            .del(&format!("posts:author:{author_id}:lim:{lim}"))
            .await;
        }
        self.cache.del("feed:latest").await;
        Ok(post)
    }

    // ── CHAT GETTERS / SETTERS ───────────────────────────────────────────────

    /// Setter: create a new chat room.
    pub async fn create_chat(
        &self,
        author_id: i32,
        title: Option<&str>,
        content: &str,
    ) -> Result<Chat, StoreError> {
        Ok(self.db.create_chat(author_id, title, content).await?)
    }

    /// Setter: add a member to a chat.
    pub async fn add_chat_member(
        &self,
        chat_id: i32,
        member_id: i32,
    ) -> Result<(), StoreError> {
        Ok(self.db.add_chat_member(chat_id, member_id).await?)
    }

    // ── MESSAGE GETTERS / SETTERS ────────────────────────────────────────────

    /// Getter: paginated message history for a chat.
    /// Short TTL (60 s) because chats are write-heavy.
    pub async fn get_chat_messages(
        &self,
        chat_id: i32,
        limit: i64,
    ) -> Result<Vec<Message>, StoreError> {
        let ck = format!("chat:msgs:{chat_id}:lim:{limit}");

        if let Some(msgs) = Self::cache_get::<Vec<Message>>(&self.cache, &ck).await {
            return Ok(msgs);
        }

        let hits = self
        .cache
        .incr_counter(&format!("access:chat:msgs:{chat_id}"), COUNTER_TTL)
        .await;
        let msgs = self.db.get_chat_messages(chat_id, limit).await?;

        if hits >= CACHE_THRESHOLD {
            Self::cache_set(&self.cache, &ck, &msgs, 60).await;
        }
        Ok(msgs)
    }

    /// Setter: send a message, invalidate the chat's message cache.
    pub async fn send_message(
        &self,
        chat_id: i32,
        author_id: i32,
        content: &str,
    ) -> Result<Message, StoreError> {
        let msg = self.db.send_message(chat_id, author_id, content).await?;
        for lim in [20i64, 50, 100] {
            self.cache
            .del(&format!("chat:msgs:{chat_id}:lim:{lim}"))
            .await;
        }
        Ok(msg)
    }
}
