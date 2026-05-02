use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use rocket::form::Form;
use rocket::fs::{NamedFile, TempFile};
use rocket::http::{ContentType, Status};
use rocket::response::content::RawJson;
use rocket::{post, get, State};
use serde_json::json;
use tokio::sync::RwLock;

use crate::secure;

const TOKEN_TTL_SECS: u64  = 300;
pub const UPLOADS_DIR: &str = "./uploads";

// ─── Upload token store ───────────────────────────────────────────────────────

pub struct UploadTokenStore {
    inner: RwLock<HashMap<String, (i32, Instant)>>,
}

impl UploadTokenStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { inner: RwLock::new(HashMap::new()) })
    }

    pub async fn create_token(&self, user_id: i32) -> String {
        let token = secure::get_token(24);
        let mut map = self.inner.write().await;
        map.retain(|_, (_, issued)| issued.elapsed().as_secs() < TOKEN_TTL_SECS);
        map.insert(token.clone(), (user_id, Instant::now()));
        token
    }

    pub async fn consume_token(&self, token: &str) -> Option<i32> {
        let mut map = self.inner.write().await;
        match map.remove(token) {
            Some((uid, issued)) if issued.elapsed().as_secs() < TOKEN_TTL_SECS => Some(uid),
            _ => None,
        }
    }
}

// ─── Multipart form ───────────────────────────────────────────────────────────

#[derive(FromForm)]
pub struct UploadForm<'v> {
    pub token: String,
    pub file:  TempFile<'v>,
}

// ─── Разрешённые типы файлов ──────────────────────────────────────────────────

fn allowed_ext(ct: &ContentType) -> Option<&'static str> {
    match (ct.top().as_str(), ct.sub().as_str()) {
        ("image", "jpeg")               => Some("jpg"),
        ("image", "png")                => Some("png"),
        ("image", "gif")                => Some("gif"),
        ("image", "webp")               => Some("webp"),
        ("video", "mp4")                => Some("mp4"),
        ("application", "x-bittorrent") => Some("torrent"),
        _                               => None,
    }
}

// ─── POST /api/upload ─────────────────────────────────────────────────────────

#[post("/upload", data = "<form>")]
pub async fn upload(
    mut form:    Form<UploadForm<'_>>,
    token_store: &State<Arc<UploadTokenStore>>,
) -> (Status, RawJson<String>) {
    macro_rules! err {
        ($status:expr, $msg:expr) => {
            return ($status, RawJson(json!({ "error": $msg }).to_string()))
        };
    }

    if token_store.consume_token(&form.token).await.is_none() {
        err!(Status::Unauthorized, "invalid_or_expired_token");
    }

    let ct = form.file.content_type().cloned().unwrap_or(ContentType::Binary);
    let ext = match allowed_ext(&ct) {
        Some(e) => e,
        None    => err!(Status::BadRequest, "unsupported_file_type"),
    };

    let filename = format!("{}.{}", secure::get_token(16), ext);
    let dest     = Path::new(UPLOADS_DIR).join(&filename);

    // ----- копируем через tokio чтобы избежать cross-device link (os error 18) -----
    let tmp_path = match form.file.path() {
        Some(p) => p.to_path_buf(),
        None    => err!(Status::InternalServerError, "no_temp_path"),
    };

    if let Err(e) = tokio::fs::copy(&tmp_path, &dest).await {
        error!("upload: copy failed: {e}");
        err!(Status::InternalServerError, "save_failed");
    }

    let url = format!("/api/files/{filename}");
    (Status::Ok, RawJson(json!({ "url": url, "filename": filename }).to_string()))
}

// ─── DELETE /api/files/<n> ────────────────────────────────────────────────────

#[rocket::delete("/files/<name>")]
pub async fn delete_file(name: &str) -> Status {
    let dot_count = name.matches('.').count();
    let chars_ok  = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');

    if !chars_ok || dot_count != 1 || name.starts_with('.') {
        return Status::BadRequest;
    }

    let path: PathBuf = [UPLOADS_DIR, name].iter().collect();
    match tokio::fs::remove_file(&path).await {
        Ok(_)  => Status::Ok,
        Err(_) => Status::NotFound,
    }
}

// ─── GET /api/files/<n> ───────────────────────────────────────────────────────

#[get("/files/<n>")]
pub async fn serve_file(n: &str) -> Option<NamedFile> {
    let dot_count = n.matches('.').count();
    let chars_ok  = n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');

    if !chars_ok || dot_count != 1 || n.starts_with('.') {
        return None;
    }

    let path: PathBuf = [UPLOADS_DIR, n].iter().collect();
    NamedFile::open(path).await.ok()
}
