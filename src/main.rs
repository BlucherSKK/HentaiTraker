#[macro_use] extern crate rocket;
use rocket::response::content::{RawHtml, RawJavaScript, RawJson};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use tokio::time::{sleep, Duration};
use rocket::http::Header;
use std::io::Cursor;
use rocket::response::stream::ReaderStream;


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

    let stream = ReaderStream! {
        let mut offset = 0;
        let total = app_content.len();
        while offset < total {
            let end = std::cmp::min(offset + chunk_size, total);
            let chunk = app_content[offset..end].to_vec();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            yield Cursor::new(chunk);
            offset = end;
        }
    };

    StreamWithLength(stream, total_len)
}




#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, app])
}
