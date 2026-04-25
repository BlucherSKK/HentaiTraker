#[macro_use] extern crate rocket;

use std::io::Cursor;
use std::sync::Arc;

use rocket::http::Header;
use rocket::response::content::{RawHtml, RawJson};
use rocket::response::stream::ReaderStream;
use rocket::response::{self, Responder, Response};
use rocket::{Request, State};
use tokio::time::Duration;

mod db;
mod handle;
mod hnts;
mod secure;

mod admin;

mod upload;
use upload::UploadTokenStore;

use db::Store;
use handle::registry::SessionRegistry;
use hnts::HntsState;

pub struct StreamWithLength<R>(R, u64);

impl<'r, R: Responder<'r, 'r>> Responder<'r, 'r> for StreamWithLength<R> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'r> {
        Response::build_from(self.0.respond_to(req)?)
        .header(Header::new("Content-Length", self.1.to_string()))
        .ok()
    }
}

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(include_str!("./loader.min.html"))
}

#[get("/app")]
fn app_js() -> StreamWithLength<ReaderStream![Cursor<Vec<u8>>]> {
    let data  = include_str!("./app.min.js").as_bytes().to_vec();
    let total = data.len() as u64;
    let delay = if cfg!(feature = "QA") { 100_000u64 } else { 0 };
    let stream = ReaderStream! {
        let mut off = 0usize;
        let len = data.len();
        while off < len {
            let end = (off + 1024).min(len);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            yield Cursor::new(data[off..end].to_vec());
            off = end;
        }
    };
    StreamWithLength(stream, total)
}

#[get("/terminal")]
fn terminal_js() -> StreamWithLength<ReaderStream![Cursor<Vec<u8>>]> {
    let data  = include_str!("./terminal.min.js").as_bytes().to_vec();
    let total = data.len() as u64;
    let stream = ReaderStream! {
        let mut off = 0usize;
        let len = data.len();
        while off < len {
            let end = (off + 1024).min(len);
            yield Cursor::new(data[off..end].to_vec());
            off = end;
        }
    };
    StreamWithLength(stream, total)
}

#[get("/app.min.js.map")]
fn app_map() -> StreamWithLength<ReaderStream![Cursor<Vec<u8>>]> {
    let data  = include_str!("./app.min.js.map").as_bytes().to_vec();
    let total = data.len() as u64;
    let stream = ReaderStream! {
        let mut off = 0usize;
        let len = data.len();
        while off < len {
            let end = (off + 1024).min(len);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            yield Cursor::new(data[off..end].to_vec());
            off = end;
        }
    };
    StreamWithLength(stream, total)
}

#[get("/getfeed")]
async fn get_feed(store: &State<Arc<Store>>) -> RawJson<String> {
    match store.get_latest_posts(20).await {
        Ok(posts) => RawJson(serde_json::to_string(&posts).unwrap_or_else(|_| "[]".into())),
        Err(e) => {
            error!("get_feed: {e}");
            RawJson("[]".into())
        }
    }
}

#[rocket::main]
async fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();

    tokio::fs::create_dir_all(upload::UPLOADS_DIR)
    .await
    .expect("cannot create uploads dir");

    let db_url    = std::env::var("DATABASE_URL").expect("DATABASE_URL не задан");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL не задан");

    let store = Arc::new(
        Store::init(&db_url, &redis_url)
        .await
        .expect("Store init failed"),
    );

    // Bootstrap: ensure user ID=1 always has the admin role (direct SQL, bypasses permission check)
    if let Ok(Some(user)) = store.get_user(1).await {
        let roles = user.roles.as_deref().unwrap_or("");
        if !roles.split(',').any(|r| r.trim() == "admin") {
            let new_roles = if roles.is_empty() {
                "admin".to_string()
            } else {
                format!("{},admin", roles)
            };
            match store.set_roles_direct(1, &new_roles).await {
                Ok(_)  => info!("пользователю ID=1 выдана роль admin"),
                Err(e) => error!("bootstrap admin role: {e}"),
            }
        }
    }

    let hnts = HntsState::new();
    hnts.start_auto_refresh(Duration::from_secs(15 * 60));

    rocket::build()
    .manage(Arc::clone(&store))
    .manage(hnts)
    .manage(SessionRegistry::new())
    .manage(UploadTokenStore::new())
    .mount("/",         routes![index, app_js, app_map, terminal_js])
    .mount("/api",      routes![get_feed, upload::upload, upload::serve_file])
    .mount("/api/hnts", routes![hnts::get_token, handle::socket::ws])
    .launch()
    .await
    .expect("Rocket crashed");
}
