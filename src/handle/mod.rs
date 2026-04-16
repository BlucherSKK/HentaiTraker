use crate::db::Post;
use chrono::Utc; // Понадобится для генерации текущего времени

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


const SERIALIZE_TAGS_RESOLUTION: u8 = 3;

impl Post {
    pub fn new(
        author_id: i32,
        title: Option<String>,
        content: String,
        tags: Vec<Tags>
    ) -> Self {
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

    fn serialize_attachments()

}
