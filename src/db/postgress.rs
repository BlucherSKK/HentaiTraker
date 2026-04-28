use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Executor;
use std::time::Duration;

use crate::db::roles::{Role, init_roles};
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

    // ----- users -----

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

    pub async fn insert_user(&self, name: &str, pass: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_insert_user($1, $2)")
        .bind(name)
        .bind(pass)
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
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM db_update_user($1, $2, $3, $4, $5, $6)"
        )
        .bind(target_id)
        .bind(modifier_id)
        .bind(name)
        .bind(pass)
        .bind(avatar)
        .bind(tags)
        .fetch_optional(&self.pool)
        .await
    }

    // ----- roles -----

    // В db.sql нужна функция db_set_user_roles — см. ниже.
    pub async fn set_user_roles(&self, user_id: i32, role_ids: &[i32]) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT db_set_user_roles($1, $2)")
        .bind(user_id)
        .bind(role_ids)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user_roles(&self, user_id: i32) -> Result<Vec<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM db_get_user_roles($1)")
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_roles(&self) -> Result<Vec<Role>, sqlx::Error> {
        sqlx::query_as::<_, Role>("SELECT * FROM db_get_roles()")
        .fetch_all(&self.pool)
        .await
    }

    pub async fn assign_role(&self, user_id: i32, role_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT db_assign_role($1, $2)")
        .bind(user_id)
        .bind(role_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_role(&self, user_id: i32, role_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT db_revoke_role($1, $2)")
        .bind(user_id)
        .bind(role_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn user_has_permission(&self, user_id: i32, permission: i32) -> Result<bool, sqlx::Error> {
        let row: (bool,) = sqlx::query_as("SELECT db_user_has_permission($1, $2)")
        .bind(user_id)
        .bind(permission)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    // ----- settings -----

    pub async fn get_settings(&self, user_id: i32) -> Result<Option<String>, sqlx::Error> {
        let row: Option<(Option<String>,)> = sqlx::query_as("SELECT db_get_settings($1)")
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|r| r.0))
    }

    pub async fn set_settings(&self, user_id: i32, settings: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM db_set_settings($1, $2)")
        .bind(user_id)
        .bind(settings)
        .fetch_optional(&self.pool)
        .await
    }

    // ----- posts -----

    pub async fn get_post_by_id(&self, id: i32) -> Result<Option<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_post(&self, author_id: i32, title: Option<&str>, content: &str, files: Option<&str>, tags: Option<&str>) -> Result<Post, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_create_post($1, $2, $3, $4, $5)")
        .bind(author_id).bind(title).bind(content).bind(files).bind(tags)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_latest_posts(&self, limit: i64) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_latest_posts($1)")
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_posts_by_author(&self, author_id: i32, limit: i32) -> Result<Vec<Post>, sqlx::Error> {
        sqlx::query_as::<_, Post>("SELECT * FROM db_get_posts_by_author($1, $2)")
        .bind(author_id).bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // ----- chats -----


    pub async fn get_chat_by_id(&self, id: i32) -> Result<Option<Chat>, sqlx::Error> {
        sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = $1")
        .bind(id)
        .fetch_optional(&self.pool)
        .await
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

    pub async fn is_chat_member(&self, chat_id: i32, member_id: i32) -> Result<bool, sqlx::Error> {
        let row: (bool,) = sqlx::query_as("SELECT db_is_chat_member($1, $2)")
        .bind(chat_id).bind(member_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    // ----- messages -----

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

    // ----- tests -----

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
        let user = db.insert_user("test_user", "password123").await.unwrap();
        assert_eq!(user.name, "test_user");
        let found = db.get_user_by_id(user.id).await.unwrap().expect("User missing");
        assert_eq!(found.id, user.id);
        let by_name = db.get_user_by_name("test_user").await.unwrap().expect("User by name missing");
        assert_eq!(by_name.id, user.id);
    }

    #[tokio::test]
    async fn test_chat_join_flow() {
        let db = setup_db().await;
        let user = db.insert_user("joiner", "pass").await.unwrap();
        let chat = db.create_chat(user.id, Some("Room"), "desc").await.unwrap();
        assert!(!db.is_chat_member(chat.id, user.id).await.unwrap());
        db.add_chat_member(chat.id, user.id).await.unwrap();
        assert!(db.is_chat_member(chat.id, user.id).await.unwrap());
    }
}
