use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex, mpsc};
use serde_json::Value;
use super::session::Session;

pub struct SessionEntry {
    pub session_id:   String,
    pub user_id:      i32,
    pub broadcast_tx: mpsc::Sender<Value>,
}

#[derive(Clone, Default)]
pub struct SessionRegistry {
    chats: Arc<RwLock<HashMap<i32, Vec<SessionEntry>>>>,
    users: Arc<RwLock<HashMap<i32, Vec<Arc<Mutex<Session>>>>>>,
}

impl SessionRegistry {
    pub fn new() -> Self { Self::default() }

    pub async fn join(&self, chat_id: i32, entry: SessionEntry) {
        self.chats.write().await
        .entry(chat_id)
        .or_default()
        .push(entry);
    }

    pub async fn register_user_session(&self, user_id: i32, session: Arc<Mutex<Session>>) {
        self.users.write().await
        .entry(user_id)
        .or_default()
        .push(session);
    }

    pub async fn leave(&self, session_id: &str) {
        let mut chats = self.chats.write().await;
        for entries in chats.values_mut() {
            entries.retain(|e| e.session_id != session_id);
        }
        chats.retain(|_, v| !v.is_empty());

        let mut users = self.users.write().await;
        for sessions in users.values_mut() {
            sessions.retain(|s| {
                if let Ok(g) = s.try_lock() { g.id != session_id } else { true }
            });
        }
        users.retain(|_, v| !v.is_empty());
    }

    pub async fn broadcast_to_chat(
        &self,
        chat_id:         i32,
        json:            Value,
        exclude_session: Option<&str>,
    ) {
        let chats = self.chats.read().await;
        let Some(entries) = chats.get(&chat_id) else { return };
        for entry in entries {
            if exclude_session.map_or(true, |sid| sid != entry.session_id) {
                let _ = entry.broadcast_tx.send(json.clone()).await;
            }
        }
    }

    pub async fn notify_roles_updated(
        &self,
        user_id:     i32,
        roles:       &[String],
        permissions: &[i32],
    ) {
        let users = self.users.read().await;
        let Some(sessions) = users.get(&user_id) else { return };

        let msg = serde_json::json!({
            "event":       "roles_updated",
            "roles":       roles,
            "permissions": permissions,
        });

        for session in sessions {
            let mut s = session.lock().await;
            s.permissions = permissions.to_vec();
            s.send_encrypted(&msg).await;
        }
    }
}
