use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use serde_json::Value;

/// Запись об одной активной WS-сессии, подписанной на чат.
pub struct SessionEntry {
    pub session_id:   String,
    pub user_id:      i32,
    /// Sender-конец broadcast-канала сессии.
    /// Receiver читает воркер, который шифрует и отправляет JSON клиенту.
    pub broadcast_tx: mpsc::Sender<Value>,
}

/// Реестр сессий по чатам. Shared-state через Rocket.
/// Clone — это дешёвый Arc-clone.
#[derive(Clone, Default)]
pub struct SessionRegistry {
    inner: Arc<RwLock<HashMap<i32, Vec<SessionEntry>>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Подписать сессию на чат (вызывается после логина).
    pub async fn join(&self, chat_id: i32, entry: SessionEntry) {
        self.inner
        .write()
        .await
        .entry(chat_id)
        .or_default()
        .push(entry);
    }

    /// Удалить все подписки сессии (вызывается при отключении).
    pub async fn leave(&self, session_id: &str) {
        let mut map = self.inner.write().await;
        for entries in map.values_mut() {
            entries.retain(|e| e.session_id != session_id);
        }
        map.retain(|_, v| !v.is_empty());
    }

    /// Разослать JSON всем сессиям чата, кроме exclude_session.
    /// Каждая сессия шифрует сообщение своим ключом в своём воркере.
    pub async fn broadcast_to_chat(
        &self,
        chat_id:         i32,
        json:            Value,
        exclude_session: Option<&str>,
    ) {
        let map = self.inner.read().await;
        let Some(entries) = map.get(&chat_id) else { return };
        for entry in entries {
            if exclude_session.map_or(true, |sid| sid != entry.session_id) {
                let _ = entry.broadcast_tx.send(json.clone()).await;
            }
        }
    }
}
