use chrono::NaiveDateTime;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Executor, FromRow};
use std::time::Duration;

#[derive(Debug, FromRow)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub pass: String,
    pub last_visit: NaiveDateTime,
    pub roles: Option<String>,
    pub avatar: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct Post {
    pub id: i32,
    pub title: Option<String>,
    pub content: String,
    pub files: Option<String>,
    pub author_id: i32,
    pub time: NaiveDateTime,
    pub tags: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct Chat {
    pub id: i32,
    pub title: Option<String>,
    pub content: String,
    pub images: Option<String>,
    pub author_id: i32,
    pub time: NaiveDateTime,
}

#[derive(Debug, FromRow)]
pub struct Message {
    pub id: i32,
    pub content: String,
    pub files: Option<String>,
    pub author_id: i32,
    pub chat_id: i32,
    pub time: NaiveDateTime,
}

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn init(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(url)
        .await?;

        let db = Self { pool };
        db.pool.execute(include_str!("./db.sql")).await?;
        db.setup_schema().await?;

        Ok(db)
    }

    async fn setup_schema(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT init_db_schema();")
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- USERS ---

    pub async fn create_user_simple(&self, username: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO users (username) VALUES ($1)")
        .bind(username)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert_user(&self, name: &str, pass: &str, roles: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (name, pass, last_visit, roles)
        VALUES ($1, $2, NOW(), $3) RETURNING *",
        )
        .bind(name)
        .bind(pass)
        .bind(roles)
        .fetch_one(&self.pool)
        .await
    }

    // --- POSTS ---

    pub async fn get_posts_by_author(&self, author_id: i32) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE author_id = $1 ORDER BY time DESC")
        .bind(author_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_latest_post_before(&self, time: NaiveDateTime) -> Result<Option<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>(
            "SELECT * FROM posts WHERE time < $1 ORDER BY time DESC LIMIT 1",
        )
        .bind(time)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_latest_post_before_now(&self) -> Result<Option<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>(
            "SELECT * FROM posts WHERE time < NOW() ORDER BY time DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert_post(
        &self,
        author_id: i32,
        title: Option<&str>,
        content: &str,
    ) -> Result<Post, sqlx::Error> {
        sqlx::query_as::<_, Post>(
            "INSERT INTO posts (title, content, author_id, time)
        VALUES ($1, $2, $3, NOW()) RETURNING *",
        )
        .bind(title)
        .bind(content)
        .bind(author_id)
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
        sqlx::query_as::<_, Chat>(
            "INSERT INTO chats (title, content, author_id, time)
        VALUES ($1, $2, $3, NOW()) RETURNING *",
        )
        .bind(title)
        .bind(content)
        .bind(author_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn add_chat_member(&self, chat_id: i32, member_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO cross_chat_members (chat_id, member_id) VALUES ($1, $2)")
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
        sqlx::query_as::<_, Message>(
            "INSERT INTO msg (content, author_id, chat_id, time)
        VALUES ($1, $2, $3, NOW()) RETURNING *",
        )
        .bind(content)
        .bind(author_id)
        .bind(chat_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_chat_messages(&self, chat_id: i32, limit: i64) -> Result<Vec<Message>, sqlx::Error> {
        sqlx::query_as::<_, Message>(
            "SELECT * FROM msg WHERE chat_id = $1 ORDER BY time DESC LIMIT $2",
        )
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
        let (name, pass, role) = ("test_user", "password123", "admin");

        let user = db.insert_user(name, pass, role).await.unwrap();
        assert_eq!(user.name, name);
        assert_eq!(user.roles.as_deref(), Some(role));

        let found = db.get_user_by_id(user.id).await.unwrap().expect("User missing");
        assert_eq!(found.id, user.id);
    }

    #[tokio::test]
    async fn test_post_flow() {
        let db = setup_db().await;
        let author = db.insert_user("author", "pass", "user").await.unwrap();

        let title = "My Post";
        let post = db.insert_post(author.id, Some(title), "Content").await.unwrap();
        assert_eq!(post.author_id, author.id);

        let posts = db.get_posts_by_author(author.id).await.unwrap();
        assert!(!posts.is_empty());

        let latest = db.get_latest_post_before_now().await.unwrap();
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
