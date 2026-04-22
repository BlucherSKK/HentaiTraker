use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use rocket::serde::{Serialize, json::Json};
use rocket::{get, State};
use crate::secure;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TokenResponse {
    token: String,
}

#[derive(Clone)]
pub struct HntsState {
    tokens: Arc<RwLock<TokenPair>>,
}

struct TokenPair {
    current: String,
    previous: Option<String>,
}

impl TokenPair {
    fn new() -> Self {
        Self {
            current: secure::get_token(16),
            previous: None,
        }
    }

    fn refresh(&mut self) {
        self.previous = Some(std::mem::replace(&mut self.current, secure::get_token(16)));
    }
}

impl HntsState {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(TokenPair::new())),
        }
    }

    pub fn start_auto_refresh(&self, period: Duration) {
        let state = Arc::clone(&self.tokens);
        tokio::spawn(async move {
            let mut ticker = interval(period);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                state.write().await.refresh();
            }
        });
    }

    pub async fn is_valid(&self, token: &str) -> bool {
        let pair = self.tokens.read().await;
        pair.current == token || pair.previous.as_deref() == Some(token)
    }

    /// Дешифрует данные текущим ВТНС, при неудаче — предыдущим.
    /// Возвращает None если ни один ключ не подошёл.
    pub async fn try_decrypt(&self, data: &[u8]) -> Option<Vec<u8>> {
        let pair = self.tokens.read().await;
        secure::decrypt(&pair.current, data)
        .or_else(|| pair.previous.as_deref().and_then(|prev| secure::decrypt(prev, data)))
    }
}

#[get("/gettoken")]
pub async fn get_token(state: &State<HntsState>) -> Json<TokenResponse> {
    let pair = state.tokens.read().await;
    Json(TokenResponse {
        token: pair.current.clone(),
    })
}
