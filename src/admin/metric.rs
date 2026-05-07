// src/server_state.rs
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Глобальные метрики сервера.
/// Arc<RwLock<...>> внутри — дешёвый Clone, подходит как Rocket managed state.
#[derive(Clone)]
pub struct ServerState {
    inner: Arc<RwLock<Metrics>>,
    started_at: Instant,
    pub sidebar_post_id: Arc<RwLock<Option<i32>>>,
}

struct Metrics {
    /// Текущее число активных WS-соединений (не обязательно аутентифицированных).
    pub connections_now: u64,
    /// Пиковое число одновременных соединений за всё время работы.
    pub connections_peak: u64,
    /// Всего соединений за время работы сервера (монотонно растёт).
    pub connections_total: u64,
    /// Аутентифицированных пользователей сейчас (у кого есть user_id).
    pub users_online: u64,
    /// Всего сообщений отправлено за время работы.
    pub messages_total: u64,
    /// Всего загрузок файлов за время работы.
    pub uploads_total: u64,
}

impl Metrics {
    fn new() -> Self {
        Self {
            connections_now:   0,
            connections_peak:  0,
            connections_total: 0,
            users_online:      0,
            messages_total:    0,
            uploads_total:     0,
        }
    }
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            inner:      Arc::new(RwLock::new(Metrics::new())),
            started_at: Instant::now(),
             sidebar_post_id: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_sidebar_post_id(&self) -> Option<i32> {
        *self.sidebar_post_id.read().await
    }

    pub async fn set_sidebar_post_id(&self, id: i32) {
        *self.sidebar_post_id.write().await = Some(id);
    }

    // ── соединения ──────────────────────────────────────────────────────────

    pub async fn on_connect(&self) {
        let mut m = self.inner.write().await;
        m.connections_now   += 1;
        m.connections_total += 1;
        if m.connections_now > m.connections_peak {
            m.connections_peak = m.connections_now;
        }
    }

    pub async fn on_disconnect(&self) {
        let mut m = self.inner.write().await;
        m.connections_now = m.connections_now.saturating_sub(1);
    }

    // ── аутентифицированные пользователи ────────────────────────────────────

    pub async fn on_user_authenticated(&self) {
        self.inner.write().await.users_online += 1;
    }

    pub async fn on_user_left(&self) {
        let mut m = self.inner.write().await;
        m.users_online = m.users_online.saturating_sub(1);
    }

    // ── события ─────────────────────────────────────────────────────────────

    pub async fn on_message_sent(&self) {
        self.inner.write().await.messages_total += 1;
    }

    pub async fn on_upload(&self) {
        self.inner.write().await.uploads_total += 1;
    }

    // ── снимок состояния ────────────────────────────────────────────────────

    pub async fn snapshot(&self) -> StateSnapshot {
        let m = self.inner.read().await;
        StateSnapshot {
            uptime_secs:        self.started_at.elapsed().as_secs(),
            connections_now:    m.connections_now,
            connections_peak:   m.connections_peak,
            connections_total:  m.connections_total,
            users_online:       m.users_online,
            messages_total:     m.messages_total,
            uploads_total:      m.uploads_total,
        }
    }

    pub async fn get_users_online(&self) -> u64 {
        self.inner.read().await.users_online
    }
}

pub struct StateSnapshot {
    pub uptime_secs:       u64,
    pub connections_now:   u64,
    pub connections_peak:  u64,
    pub connections_total: u64,
    pub users_online:      u64,
    pub messages_total:    u64,
    pub uploads_total:     u64,
}

impl StateSnapshot {
    /// Форматированная строка для вывода в терминал.
    pub fn format(&self) -> String {
        let h = self.uptime_secs / 3600;
        let m = (self.uptime_secs % 3600) / 60;
        let s = self.uptime_secs % 60;
        format!(
            "┌─ Server Stats ─────────────────────┐#NL#\
            │  Uptime          {h:>3}h {m:02}m {s:02}s        │#NL#\
            │  WS online       {cn:>6}               │#NL#\
            │  WS peak         {cp:>6}               │#NL#\
            │  WS total        {ct:>6}               │#NL#\
            │  Users online    {uo:>6}               │#NL#\
            │  Messages sent   {ms:>6}               │#NL#\
            │  Uploads         {ul:>6}               │#NL#\
            └────────────────────────────────────┘",
            h = h,
            m = m,
            s = s,
            cn = self.connections_now,
            cp = self.connections_peak,
            ct = self.connections_total,
            uo = self.users_online,
            ms = self.messages_total,
            ul = self.uploads_total,
        )
    }
}
