use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Executor;
use std::time::Duration;
use chrono::NaiveDateTime;
use crate::db::{User, Post, Chat, Message};

#[derive(Clone)]
pub struct Database { pool: PgPool }

impl Database {
    pub async fn init(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
        .max_connections(30)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await?;
        pool.execute(include_str!("./db.sql")).await?;
        Ok(Self { pool })
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    pub async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_get_user_by_id($1)")
        .bind(id).fetch_optional(&self.pool).await
    }

    /// Новый: поиск по имени для логина.
    pub async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_get_user_by_name($1)")
        .bind(name).fetch_optional(&self.pool).await
    }

    pub async fn insert_user(&self, name: &str, pass: &str, roles: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_insert_user($1, $2, $3)")
        .bind(name).bind(pass).bind(roles).fetch_one(&self.pool).await
    }

    // ── Posts ─────────────────────────────────────────────────────────────────

    pub async fn get_posts_by_author(&self, author_id: i32, limit: i64) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_posts_by_author($1, $2)")
        .bind(author_id).bind(limit).fetch_all(&self.pool).await
    }

    pub async fn get_latest_post_before(&self, time: NaiveDateTime) -> Result<Option<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_latest_post_before($1)")
        .bind(time).fetch_optional(&self.pool).await
    }

    pub async fn get_latest_post_now(&self) -> Result<Option<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_latest_post_now()")
        .fetch_optional(&self.pool).await
    }

    /// Новый: последние N постов для ленты.
    pub async fn get_latest_posts(&self, limit: i64) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_latest_posts($1)")
        .bind(limit).fetch_all(&self.pool).await
    }

    pub async fn insert_post(&self, author_id: i32, title: Option<&str>, content: &str) -> Result<Post, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_insert_post($1, $2, $3)")
        .bind(author_id).bind(title).bind(content).fetch_one(&self.pool).await
    }

    // ── Chats ─────────────────────────────────────────────────────────────────

    pub async fn create_chat(&self, author_id: i32, title: Option<&str>, content: &str) -> Result<Chat, sqlx::Error> {
        sqlx::query_as::<_, Chat>("SELECT * FROM db_create_chat($1, $2, $3)")
        .bind(author_id).bind(title).bind(content).fetch_one(&self.pool).await
    }

    pub async fn add_chat_member(&self, chat_id: i32, member_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT db_add_chat_member($1, $2)")
        .bind(chat_id).bind(member_id).execute(&self.pool).await?;
        Ok(())
    }

    /// Новый: чаты пользователя через таблицу chat_members.
    pub async fn get_user_chats(&self, user_id: i32) -> Result<Vec<Chat>, sqlx::Error> {
        sqlx::query_as::<_, Chat>("SELECT * FROM db_get_user_chats($1)")
        .bind(user_id).fetch_all(&self.pool).await
    }

    // ── Messages ──────────────────────────────────────────────────────────────

    pub async fn send_message(&self, chat_id: i32, author_id: i32, content: &str) -> Result<Message, sqlx::Error> {
        sqlx::query_as::<_, Message>("SELECT * FROM db_send_message($1, $2, $3)")
        .bind(chat_id).bind(author_id).bind(content).fetch_one(&self.pool).await
    }

    pub async fn get_chat_messages(&self, chat_id: i32, limit: i64) -> Result<Vec<Message>, sqlx::Error> {
        sqlx::query_as::<_, Message>("SELECT * FROM db_get_chat_messages($1, $2)")
        .bind(chat_id).bind(limit).fetch_all(&self.pool).await
    }
}
