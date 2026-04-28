use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub mod roles;
pub use roles::{Permission, Role, UserRole};

pub mod postgress;
pub mod redis;

use postgress::Database;
use redis::{RedisClient, RedisInitError};

// ----- domain types -----

#[derive(Debug, Serialize, Deserialize)]
pub enum Tags { Hentai, Any }

#[derive(Debug, Serialize, Deserialize)]
pub enum AttachmentType { Torrent, Png, Gif, Jpg }

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id:         i32,
    pub name:       String,
    pub pass:       String,
    pub last_visit: NaiveDateTime,
    pub avatar:     Option<String>,
    pub tags:       Option<String>,
    pub settings:   Option<String>,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Post {
    pub id:        i32,
    pub title:     Option<String>,
    pub content:   String,
    pub files:     Option<String>,
    pub author_id: i32,
    pub time:      NaiveDateTime,
    pub tags:      Option<String>,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Chat {
    pub id:        i32,
    pub title:     Option<String>,
    pub content:   String,
    pub images:    Option<String>,
    pub author_id: i32,
    pub time:      NaiveDateTime,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Message {
    pub id:        i32,
    pub content:   String,
    pub files:     Option<String>,
    pub author_id: i32,
    pub chat_id:   i32,
    pub time:      NaiveDateTime,
}

// ----- post builder -----

const SERIALIZE_TAGS_RESOLUTION: u8 = 3;

impl Post {
    pub fn new(author_id: i32, title: Option<String>, content: String, tags: Vec<Tags>) -> Self {
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

// ----- error -----

#[derive(Debug)]
pub enum StoreError {
    Db(sqlx::Error),
    Redis(RedisInitError),
    Json(serde_json::Error),
    NotFound,
    Unauthorized
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

// ----- store -----

#[derive(Clone)]
pub struct Store {
    pub db:    Database,
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

    // ----- users -----

    pub async fn db_is_member(&self, chat_id: i32, member_id: i32) -> Result<bool, StoreError> {
        Ok(self.db.is_chat_member(chat_id, member_id).await?)
    }

    pub async fn get_user(&self, id: i32) -> Result<Option<User>, StoreError> {
        let ck = format!("user:{id}");
        if let Some(u) = Self::cache_get::<User>(&self.cache, &ck).await { return Ok(Some(u)); }
        let hits = self.cache.incr_counter(&format!("access:user:{id}"), COUNTER_TTL).await;
        let user = self.db.get_user_by_id(id).await?;
        if hits >= CACHE_THRESHOLD {
            if let Some(ref u) = user {
                Self::cache_set(&self.cache, &ck, u, COUNTER_TTL).await;
            }
        }
        Ok(user)
    }

    pub async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, StoreError> {
        Ok(self.db.get_user_by_name(name).await?)
    }

    pub async fn insert_user(&self, name: &str, pass: &str) -> Result<User, StoreError> {
        Ok(self.db.insert_user(name, pass).await?)
    }

    pub async fn update_user(
        &self,
        target_id:   i32,
        modifier_id: i32,
        name:        Option<&str>,
        pass:        Option<&str>,
        avatar:      Option<&str>,
        tags:        Option<&str>,
    ) -> Result<Option<User>, StoreError> {
        let user = self.db.update_user(target_id, modifier_id, name, pass, avatar, tags).await?;
        if user.is_some() {
            self.cache.del(&format!("user:{target_id}")).await;
        }
        Ok(user)
    }

    // ----- roles -----

    pub async fn get_user_roles(&self, user_id: i32) -> Result<Vec<Role>, StoreError> {
        Ok(self.db.get_user_roles(user_id).await?)
    }

    pub async fn assign_role(&self, user_id: i32, role_id: i32) -> Result<(), StoreError> {
        self.db.assign_role(user_id, role_id).await?;
        self.cache.del(&format!("user:{user_id}")).await;
        Ok(())
    }

    pub async fn revoke_role(&self, user_id: i32, role_id: i32) -> Result<(), StoreError> {
        self.db.revoke_role(user_id, role_id).await?;
        self.cache.del(&format!("user:{user_id}")).await;
        Ok(())
    }

    pub async fn set_user_roles(
        &self,
        target_id: i32,
        role_ids:  &[i32],
    ) -> Result<Vec<Role>, StoreError> {
        self.db.set_user_roles(target_id, role_ids).await?;
        self.cache.del(&format!("user:{target_id}")).await;
        Ok(self.db.get_user_roles(target_id).await?)
    }

    pub async fn user_has_permission(&self, user_id: i32, permission: Permission) -> Result<bool, StoreError> {
        Ok(self.db.user_has_permission(user_id, permission.as_i32()).await?)
    }

    pub async fn get_roles(&self) -> Result<Vec<Role>, StoreError> {
        Ok(self.db.get_roles().await?)
    }

    // ----- settings -----

    pub async fn get_settings(&self, user_id: i32) -> Result<Option<String>, StoreError> {
        Ok(self.db.get_settings(user_id).await?)
    }

    pub async fn set_settings(&self, user_id: i32, settings: &str) -> Result<Option<User>, StoreError> {
        Ok(self.db.set_settings(user_id, settings).await?)
    }

    // ----- posts -----

    pub async fn create_post(&self, author_id: i32, title: Option<&str>, content: &str, tags: Option<&str>) -> Result<Post, StoreError> {
        Ok(self.db.create_post(author_id, title, content, tags).await?)
    }

    pub async fn get_latest_posts(&self, limit: i64) -> Result<Vec<Post>, StoreError> {
        Ok(self.db.get_latest_posts(limit).await?)
    }

    pub async fn get_posts_by_author(&self, author_id: i32, limit: i32) -> Result<Vec<Post>, StoreError> {
        Ok(self.db.get_posts_by_author(author_id, limit).await?)
    }

    // ----- chats -----

    pub async fn get_user_chats(&self, user_id: i32) -> Result<Vec<Chat>, StoreError> {
        Ok(self.db.get_user_chats(user_id).await?)
    }

    pub async fn create_chat(&self, author_id: i32, title: Option<&str>, content: &str) -> Result<Chat, StoreError> {
        Ok(self.db.create_chat(author_id, title, content).await?)
    }

    pub async fn add_chat_member(&self, chat_id: i32, member_id: i32) -> Result<(), StoreError> {
        Ok(self.db.add_chat_member(chat_id, member_id).await?)
    }

    // ----- messages -----

    pub async fn send_message(&self, chat_id: i32, author_id: i32, content: &str, files: Option<&str>) -> Result<Message, StoreError> {
        Ok(self.db.send_message(chat_id, author_id, content, files).await?)
    }

    pub async fn get_chat_messages(&self, chat_id: i32, limit: i64) -> Result<Vec<Message>, StoreError> {
        Ok(self.db.get_chat_messages(chat_id, limit).await?)
    }
}
