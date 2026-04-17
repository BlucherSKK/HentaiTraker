use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use chrono::{NaiveDateTime, Utc};
use crate::secure;

const TEMPLATE: &str = include_str!("web/loader.html");

pub struct AppState {
    pub loader: Arc<RwLock<Hntsloaderpage>>,
}

pub struct Hntsloaderpage {
    template: &'static str,
    mark: String,
    page: String,
    ref_time: NaiveDateTime,
}

impl Hntsloaderpage {
    pub fn new(template: &'static str, mark: impl Into<String>) -> Self {
        let mark = mark.into();
        let page = template.replace(&mark, &Self::generate_token());
        let ref_time = Utc::now().naive_utc();

        Self {
            template,
            mark,
            page,
            ref_time,
        }
    }

    pub fn start_auto_refresh(state: Arc<RwLock<Self>>, period: Duration) {
        tokio::spawn(async move {
            let mut ticker = interval(period);
            loop {
                ticker.tick().await;
                state.write().await.refresh();
            }
        });
    }

    pub fn get_page(&self) -> &str {
        &self.page
    }

    pub fn last_updated(&self) -> NaiveDateTime {
        self.ref_time
    }

    fn refresh(&mut self) {
        self.page = self.template.replace(&self.mark, &Self::generate_token());
        self.ref_time = Utc::now().naive_utc();
    }

    fn generate_token() -> String {
        secure::get_token(16)
    }
}
