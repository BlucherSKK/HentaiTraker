use log::{info, debug, error};
use sqlx::{Executor, FromRow};
pub mod postgress;
use chrono::Utc; // Понадобится для генерации текущего времени
use serde::Serialize;
use chrono::NaiveDateTime;


#[derive(Debug, Serialize)]
pub enum Tags {
    Hentai,
    Any
}

pub enum AttachmentType {
    Torrent,
    Png,
    Gif,
    Jpg
}

#[derive(Debug, FromRow, Serialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub pass: String,
    pub last_visit: NaiveDateTime,
    pub roles: Option<String>,
    pub avatar: Option<String>,
    pub tags: Option<Vec<Tags>>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Post {
    pub id: i32,
    pub title: Option<String>,
    pub content: String,
    pub files: Option<String>,
    pub author_id: i32,
    pub time: NaiveDateTime,
    pub tags:Option<Vec<Tags>>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Chat {
    pub id: i32,
    pub title: Option<String>,
    pub content: String,
    pub images: Option<String>,
    pub author_id: i32,
    pub time: NaiveDateTime,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Message {
    pub id: i32,
    pub content: String,
    pub files: Option<String>,
    pub author_id: i32,
    pub chat_id: i32,
    pub time: NaiveDateTime,
}




const SERIALIZE_TAGS_RESOLUTION: u8 = 3;

impl Post {
    pub fn new(
        author_id: i32,
        title: Option<String>,
        content: String,
        tags: Vec<Tags>
    ) -> Self {
        debug!(format!("new post constracting with title {}", title));
        Self {
            id: -1,
            title,
            content,
            files: None,
            author_id,
            time: Utc::now().naive_utc(),
            tags: Self::serialize_tags(tags),
        }
    }

    fn serialize_tags(tags: Vec<Tags>) -> Option<String> {
        if tags.is_empty() {
            return None;
        }

        let serialized: String = tags
        .iter()
        .map(|tag| match tag {
            Tags::Hentai => "hnt",
            Tags::Any => "any",
        })
        .collect();

        Some(format!("{}{}", SERIALIZE_TAGS_RESOLUTION, serialized).to_string())
    }

    //fn serialize_attachments()

}
