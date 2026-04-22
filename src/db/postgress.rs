use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Executor;
use std::time::Duration;
use chrono::NaiveDateTime;

use crate::db::{User, Post, Chat, Message};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn init(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
        .max_connections(30)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await?;

        // db.sql is self-contained: defines init functions, calls init_db_schema(),
        // then defines all named query functions. Safe to run on every startup.
        pool.execute(include_str!("./db.sql")).await?;

        Ok(Self { pool })
    }

    // --- USERS ---

    pub async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_get_user_by_id($1)")
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert_user(
        &self,
        name: &str,
        pass: &str,
        roles: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_insert_user($1, $2, $3)")
        .bind(name)
        .bind(pass)
        .bind(roles)
        .fetch_one(&self.pool)
        .await
    }

    // --- POSTS ---

    pub async fn get_posts_by_author(
        &self,
        author_id: i32,
        limit: i64,
    ) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_posts_by_author($1, $2)")
        .bind(author_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_latest_post_before(
        &self,
        time: NaiveDateTime,
    ) -> Result<Option<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_latest_post_before($1)")
        .bind(time)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_latest_post_now(&self) -> Result<Option<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_latest_post_now()")
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert_post(
        &self,
        author_id: i32,
        title: Option<&str>,
        content: &str,
    ) -> Result<Post, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_insert_post($1, $2, $3)")
        .bind(author_id)
        .bind(title)
        .bind(content)
        .fetch_one(&self.pool)
        .await
    }

    // --- CHATS ---

    pub async fn create_chat(
        &self,
        author_id: i32,
        title: Option<&str>,
        content: &str,
    ) -> Result<Chat, sqlx::Error> {
        sqlx::query_as::<_, Chat>("SELECT * FROM db_create_chat($1, $2, $3)")
        .bind(author_id)
        .bind(title)
        .bind(content)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn add_chat_member(&self, chat_id: i32, member_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT db_add_chat_member($1, $2)")
        .bind(chat_id)
        .bind(member_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- MESSAGES ---

    pub async fn send_message(
        &self,
        chat_id: i32,
        author_id: i32,
        content: &str,
    ) -> Result<Message, sqlx::Error> {
        sqlx::query_as::<_, Message>("SELECT * FROM db_send_message($1, $2, $3)")
        .bind(chat_id)
        .bind(author_id)
        .bind(content)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_chat_messages(
        &self,
        chat_id: i32,
        limit: i64,
    ) -> Result<Vec<Message>, sqlx::Error> {
        sqlx::query_as::<_, Message>("SELECT * FROM db_get_chat_messages($1, $2)")
        .bind(chat_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> Database {
        let url = "postgres://user:mysecretpassword@localhost:5432/test_db";
        Database::init(url).await.expect("Database initialization failed")
    }

    #[tokio::test]
    async fn test_user_flow() {
        let db = setup_db().await;
        let user = db.insert_user("test_user", "password123", "admin").await.unwrap();
        assert_eq!(user.name, "test_user");
        assert_eq!(user.roles.as_deref(), Some("admin"));

        let found = db.get_user_by_id(user.id).await.unwrap().expect("User missing");
        assert_eq!(found.id, user.id);
    }

    #[tokio::test]
    async fn test_post_flow() {
        let db = setup_db().await;
        let author = db.insert_user("author", "pass", "user").await.unwrap();
        let post = db.insert_post(author.id, Some("My Post"), "Content").await.unwrap();
        assert_eq!(post.author_id, author.id);

        let posts = db.get_posts_by_author(author.id, 50).await.unwrap();
        assert!(!posts.is_empty());

        let latest = db.get_latest_post_now().await.unwrap();
        assert!(latest.is_some());
    }

    #[tokio::test]
    async fn test_messaging_flow() {
        let db = setup_db().await;
        let user = db.insert_user("messenger", "pass", "user").await.unwrap();
        let chat = db.create_chat(user.id, Some("Dev"), "Topic").await.unwrap();
        let msg = db.send_message(chat.id, user.id, "Hi").await.unwrap();
        assert_eq!(msg.chat_id, chat.id);

        let history = db.get_chat_messages(chat.id, 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Hi");
    }

    #[tokio::test]
    async fn test_not_found() {
        let db = setup_db().await;
        let result = db.get_user_by_id(-1).await.unwrap();
        assert!(result.is_none());
    }
}
