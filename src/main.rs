#[macro_use] extern crate rocket;
use rocket::response::content::{RawHtml, RawJavaScript, RawJson};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use tokio::time::{sleep, Duration};
use rocket::http::Header;
use std::io::Cursor;
use serde_json;
use rocket::response::stream::ReaderStream;
use rocket::get;
use rocket::serde::{Serialize, Deserialize, json::Json}; // Добавили Json сюда
use std::sync::RwLock; // Используем RwLock вместо Mutex для скорости (много читателей, один писатель)
use rocket::State;
mod secure;
mod handle;
mod hnts;
mod db;

pub struct AppState {
    // Храним уже готовую JSON строку
    pub cached_feed: RwLock<String>,
}


#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PostResponse {
    pub message: String,
    pub status: String,
}

#[get("/getfeed")]
pub async fn get_feed(state: &State<AppState>) -> RawJson<String> {
    // Искусственная задержка (по желанию)
    sleep(Duration::from_secs(1)).await;

    // Читаем закэшированную строку из состояния
    let cache = state.cached_feed.read().expect("Lock failed");

    // Возвращаем как чистый JSON
    RawJson(cache.clone())
}


#[get("/")]
fn index() -> RawHtml<&'static str> {
    // Путь указывается относительно текущего .rs файла
    let html = include_str!("./web/loader.html");
    RawHtml(html)
}


pub struct StreamWithLength<R>(R, u64);

impl<'r, R: Responder<'r, 'r>> Responder<'r, 'r> for StreamWithLength<R> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        Response::build_from(self.0.respond_to(req)?)
        .header(Header::new("Content-Length", self.1.to_string()))
        .ok()
    }
}

#[get("/app")]
fn app() -> StreamWithLength<ReaderStream![Cursor<Vec<u8>>]> {
    let app_content = include_str!("./web/app.min.js").as_bytes().to_vec();
    let total_len = app_content.len() as u64;
    let chunk_size = 1024;
    let delay = if cfg!(feature = "QA") {100000} else {0};

    let stream = ReaderStream! {
        let mut offset = 0;
        let total = app_content.len();
        while offset < total {
            let end = std::cmp::min(offset + chunk_size, total);
            let chunk = app_content[offset..end].to_vec();
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            yield Cursor::new(chunk);
            offset = end;
        }
    };

    StreamWithLength(stream, total_len)
}



#[get("/app.min.js.map")]
fn appmap() -> StreamWithLength<ReaderStream![Cursor<Vec<u8>>]> {
    let app_content = include_str!("./web/app.min.js.map").as_bytes().to_vec();
    let total_len = app_content.len() as u64;
    let chunk_size = 1024;

    let stream = ReaderStream! {
        let mut offset = 0;
        let total = app_content.len();
        while offset < total {
            let end = std::cmp::min(offset + chunk_size, total);
            let chunk = app_content[offset..end].to_vec();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            yield Cursor::new(chunk);
            offset = end;
        }
    };

    StreamWithLength(stream, total_len)
}


use crate::db::Post;
use crate::db::Tags;

#[rocket::main]
async fn main() {
    env_logger::init();
    // 1. Создаем начальные данные (динамический массив)
    // Убедись, что структура Post объявлена выше или импортирована
    let initial_posts: Vec<Post> = vec![
        Post::new(1, Some("x".to_string()), "we".to_string(), vec![Tags::Hentai])
    ];

    // 2. Сериализуем массив в JSON-строку заранее
    let json_string = serde_json::to_string(&initial_posts)
    .expect("Ошибка при создании JSON");

    // 3. Создаем стейт с кэшем
    let state = AppState {
        cached_feed: RwLock::new(json_string),
    };

    let _ = rocket::build()
    .manage(state)
    .mount("/", routes![index, app, appmap])
    .mount("/api", routes![get_feed])
    .launch()
    .await
    ;
}

