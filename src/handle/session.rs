use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use rocket_ws::Message;
use serde_json::Value;
use crate::db::Store;
use crate::secure;

pub const TOKEN_LIFETIME: Duration = Duration::from_secs(20 * 60);

// ─── SessionState ─────────────────────────────────────────────────────────────

pub enum SessionState {
    /// Начальное состояние: клиент не представился.
    Entrypoint,
    /// Клиент передал зашифрованную пару ВТНС; следующие сообщения шифруются private_vtns.
    LongToken {
        public_vtns: String,
            private_vtns: String,
                issued_at: Instant,
    },
    /// Аутентификация пройдена; сообщения шифруются priv_at.
    Authenticated {
        pub_at: String,
        priv_at: String,
        issued_at: Instant,
    },
    /// pub_at скомпрометирован; клиент восстанавливает сессию через hash(priv_at).
    PrivateOnly(String),
}

// ─── Session ─────────────────────────────────────────────────────────────────

pub struct Session {
    /// Уникальный идентификатор сессии (случайный токен).
    pub id: String,
    pub state: SessionState,
    /// Канал для отправки сообщений клиенту (пишет задача-sink).
    pub tx: mpsc::Sender<Message>,
    /// Заполняется после перехода в Authenticated.
    pub user_id: Option<i32>,
    /// pub_vtns из URL — идентификатор до завершения рукопожатия.
    pub pub_vtns: String,
    pub connected_at: Instant,
    pub last_activity: Instant,
    /// Доступ к БД; задаётся извне после инициализации Store.
    pub store: Option<Arc<Store>>,
}

impl Session {
    pub fn new(tx: mpsc::Sender<Message>, pub_vtns: String) -> Self {
        let now = Instant::now();
        Self {
            id: secure::get_token(16),
            state: SessionState::Entrypoint,
            tx,
            user_id: None,
            pub_vtns,
            connected_at: now,
            last_activity: now,
            store: None,
        }
    }

    pub async fn send(&self, msg: impl Into<String>) -> Result<(), mpsc::error::SendError<Message>> {
        self.tx.send(Message::Text(msg.into())).await
    }

    pub async fn send_binary(&self, data: Vec<u8>) -> Result<(), mpsc::error::SendError<Message>> {
        self.tx.send(Message::Binary(data)).await
    }

    pub async fn send_json(&self, value: &Value) -> Result<(), mpsc::error::SendError<Message>> {
        self.tx.send(Message::Text(value.to_string())).await
    }

    /// Шифрует Value текущим ключом сессии и отправляет как бинарное.
    /// LongToken → private_vtns, Authenticated → priv_at. Остальные — нет-оп.
    pub async fn send_encrypted(&self, value: &Value) {
        let key = match &self.state {
            SessionState::LongToken { private_vtns, .. } => private_vtns.as_str(),
            SessionState::Authenticated { priv_at, .. } => priv_at.as_str(),
            _ => return,
        };
        let enc = secure::encrypt(key, value.to_string().as_bytes());
        let _ = self.send_binary(enc).await;
    }

    pub fn tokens_expired(&self) -> bool {
        match &self.state {
            SessionState::LongToken { issued_at, .. }
            | SessionState::Authenticated { issued_at, .. } => {
                issued_at.elapsed() >= TOKEN_LIFETIME
            }
            _ => false,
        }
    }
}
