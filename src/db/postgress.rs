use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Executor;
use std::time::Duration;
use chrono::NaiveDateTime;
use crate::db::roles::{Role, UserRole, init_roles};

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
        pool.execute(include_str!("./db.sql")).await?;
        init_roles(&pool).await?;
        Ok(Self { pool })
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    pub async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_get_user_by_id($1)")
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_get_user_by_name($1)")
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn insert_user(&self, name: &str, pass: &str, roles: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_insert_user($1, $2, $3)")
        .bind(name).bind(pass).bind(roles)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_user(
        &self,
        target_id:   i32,
        modifier_id: i32,
        name:        Option<&str>,
        pass:        Option<&str>,
        avatar:      Option<&str>,
        tags:        Option<&str>,
        roles:       Option<&str>,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM db_update_user($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(target_id)
        .bind(modifier_id)
        .bind(name)
        .bind(pass)
        .bind(avatar)
        .bind(tags)
        .bind(roles)
        .fetch_optional(&self.pool)
        .await
    }



    pub async fn set_roles_direct(&self, user_id: i32, roles: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET roles = $1 WHERE id = $2")
        .bind(roles)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Posts ─────────────────────────────────────────────────────────────────

    pub async fn insert_post_with_files(
        &self,
        author_id: i32,
        title:     Option<&str>,
        content:   &str,
        files:     Option<&str>,
    ) -> Result<Post, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_insert_post_with_files($1, $2, $3, $4)")
        .bind(author_id).bind(title).bind(content).bind(files)
        .fetch_one(&self.pool)
        .await
    }


    pub async fn get_posts_by_author(&self, author_id: i32, limit: i64) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_posts_by_author($1, $2)")
        .bind(author_id).bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_latest_post_before(&self, time: NaiveDateTime) -> Result<Option<Post>, sqlx::Error> {
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

    pub async fn get_latest_posts(&self, limit: i64) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_latest_posts($1)")
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn insert_post(&self, author_id: i32, title: Option<&str>, content: &str) -> Result<Post, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_insert_post($1, $2, $3)")
        .bind(author_id).bind(title).bind(content)
        .fetch_one(&self.pool)
        .await
    }


    // ── Roles ─────────────────────────────────────────────────────────────────

    pub async fn get_roles(&self) -> Result<Vec<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM db_get_roles()")
        .fetch_all(&self.pool)
        .await
    }

    pub async fn create_role(&self, name: &str, permissions: &[i32]) -> Result<Role, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM db_create_role($1, $2)")
        .bind(name).bind(permissions)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_user_roles(&self, user_id: i32) -> Result<Vec<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM db_get_user_roles($1)")
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn assign_role(&self, user_id: i32, role_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT db_assign_role($1, $2)")
        .bind(user_id).bind(role_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_role(&self, user_id: i32, role_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT db_revoke_role($1, $2)")
        .bind(user_id).bind(role_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn has_role(&self, user_id: i32, role_id: i32) -> Result<bool, sqlx::Error> {
        let row: (bool,) = sqlx::query_as("SELECT db_has_role($1, $2)")
        .bind(user_id).bind(role_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn user_has_permission(&self, user_id: i32, permission: i32) -> Result<bool, sqlx::Error> {
        let row: (bool,) = sqlx::query_as("SELECT db_user_has_permission($1, $2)")
        .bind(user_id).bind(permission)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    // get_admin_role_id — для bootstrap
    pub async fn get_role_by_name(&self, name: &str) -> Result<Option<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE name = $1")
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }

    // ── Chats ─────────────────────────────────────────────────────────────────

    pub async fn get_chat_by_id(&self, chat_id: i32) -> Result<Option<Chat>, sqlx::Error> {
        sqlx::query_as::<_, Chat>("SELECT * FROM db_get_chat_by_id($1)")
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn is_chat_member(&self, chat_id: i32, member_id: i32) -> Result<bool, sqlx::Error> {
        let row: (bool,) = sqlx::query_as("SELECT db_is_chat_member($1, $2)")
        .bind(chat_id).bind(member_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn get_user_chats(&self, member_id: i32) -> Result<Vec<Chat>, sqlx::Error> {
        sqlx::query_as::<_, Chat>("SELECT * FROM db_get_user_chats($1)")
        .bind(member_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn create_chat(&self, author_id: i32, title: Option<&str>, content: &str) -> Result<Chat, sqlx::Error> {
        sqlx::query_as::<_, Chat>("SELECT * FROM db_create_chat($1, $2, $3)")
        .bind(author_id).bind(title).bind(content)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn add_chat_member(&self, chat_id: i32, member_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT db_add_chat_member($1, $2)")
        .bind(chat_id).bind(member_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Messages ──────────────────────────────────────────────────────────────

    pub async fn send_message(&self, chat_id: i32, author_id: i32, content: &str, files: Option<&str>) -> Result<Message, sqlx::Error> {
        sqlx::query_as::<_, Message>("SELECT * FROM db_send_message($1, $2, $3, $4)")
        .bind(chat_id).bind(author_id).bind(content).bind(files)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_chat_messages(&self, chat_id: i32, limit: i64) -> Result<Vec<Message>, sqlx::Error> {
        sqlx::query_as::<_, Message>("SELECT * FROM db_get_chat_messages($1, $2)")
        .bind(chat_id).bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[cfg(test)]
    pub async fn get_pool(&self) -> &PgPool { &self.pool }
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
        let found = db.get_user_by_id(user.id).await.unwrap().expect("User missing");
        assert_eq!(found.id, user.id);
        let by_name = db.get_user_by_name("test_user").await.unwrap().expect("User by name missing");
        assert_eq!(by_name.id, user.id);
    }

    #[tokio::test]
    async fn test_chat_join_flow() {
        let db = setup_db().await;
        let user  = db.insert_user("joiner", "pass", "user").await.unwrap();
        let chat  = db.create_chat(user.id, Some("Room"), "desc").await.unwrap();

        // Initial: not a member
        assert!(!db.is_chat_member(chat.id, user.id).await.unwrap());

        db.add_chat_member(chat.id, user.id).await.unwrap();
        assert!(db.is_chat_member(chat.id, user.id).await.unwrap());

        // Idempotent
        db.add_chat_member(chat.id, user.id).await.unwrap();

        let chats = db.get_user_chats(user.id).await.unwrap();
        assert!(chats.iter().any(|c| c.id == chat.id));
    }

    #[tokio::test]
    async fn test_messaging_flow() {
        let db = setup_db().await;
        let user = db.insert_user("messenger", "pass", "user").await.unwrap();
        let chat = db.create_chat(user.id, Some("Dev"), "Topic").await.unwrap();
        let msg  = db.send_message(chat.id, user.id, "Hi").await.unwrap();
        assert_eq!(msg.chat_id, chat.id);
        let history = db.get_chat_messages(chat.id, 10).await.unwrap();
        assert_eq!(history[0].content, "Hi");
    }
}
