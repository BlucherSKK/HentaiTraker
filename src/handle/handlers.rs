use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::{Value, json};
use super::session::Session;
use super::registry::{SessionRegistry, SessionEntry};

// ─── message_create ───────────────────────────────────────────────────────────

/// Сохраняет сообщение в БД и транслирует его участникам чата.
/// Payload: { chat_id: i32, content: string }
pub async fn message_create(
    session:  Arc<Mutex<Session>>,
    data:     Value,
    registry: Arc<SessionRegistry>,
) {
    let (store, user_id, session_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id, s.id.clone())
    };

    let store   = match store   { Some(s) => s, None => return };
    let user_id = match user_id { Some(id) => id, None => return };
    let chat_id = match data["chat_id"].as_i64() {
        Some(id) => id as i32,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "missing_chat_id" })).await;
            return;
        }
    };
    let content = data["content"].as_str().unwrap_or("").trim().to_string();
    if content.is_empty() {
        let s = session.lock().await;
        s.send_encrypted(&json!({ "event": "error", "code": "empty_content" })).await;
        return;
    }

    match store.send_message(chat_id, user_id, &content).await {
        Ok(msg) => {
            let broadcast = json!({
                "event":     "new_message",
                "id":        msg.id,
                "chat_id":   msg.chat_id,
                "author_id": msg.author_id,
                "content":   msg.content,
                "time":      msg.time.to_string(),
            });
            // Транслируем другим участникам чата (каждый шифрует сам)
            registry.broadcast_to_chat(chat_id, broadcast.clone(), Some(&session_id)).await;
            // Подтверждение отправителю
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "message_sent", "message": broadcast })).await;
        }
        Err(e) => {
            log::error!("message_create db error: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

// ─── message_list ────────────────────────────────────────────────────────────

/// Возвращает историю сообщений чата.
/// Payload: { chat_id: i32, limit?: i64 }  (default = 50, max = 100)
pub async fn message_list(session: Arc<Mutex<Session>>, data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };

    let store = match store   { Some(s) => s, None => return };
    let _     = match user_id { Some(id) => id, None => return };
    let chat_id = match data["chat_id"].as_i64() {
        Some(id) => id as i32,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "missing_chat_id" })).await;
            return;
        }
    };
    let limit = data["limit"].as_i64().unwrap_or(50).clamp(1, 100);

    match store.get_chat_messages(chat_id, limit).await {
        Ok(messages) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({
                "event":    "message_list",
                "chat_id":  chat_id,
                "messages": messages,
            })).await;
        }
        Err(e) => {
            log::error!("message_list db error: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

// ─── chat_list ───────────────────────────────────────────────────────────────

/// Возвращает список чатов пользователя.
/// Payload: {}
pub async fn chat_list(session: Arc<Mutex<Session>>, _data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };

    let store   = match store   { Some(s) => s, None => return };
    let user_id = match user_id { Some(id) => id, None => return };

    match store.get_user_chats(user_id).await {
        Ok(chats) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "chat_list", "chats": chats })).await;
        }
        Err(e) => {
            log::error!("chat_list db error: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

// ─── chat_create ─────────────────────────────────────────────────────────────

/// Создаёт чат и добавляет создателя как участника.
/// Payload: { title?: string, description: string }
pub async fn chat_create(
    session:  Arc<Mutex<Session>>,
    data:     Value,
    registry: Arc<SessionRegistry>,
) {
    let (store, user_id, session_id, bcast_tx) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id, s.id.clone(), s.broadcast_tx.clone())
    };

    let store   = match store   { Some(s) => s, None => return };
    let user_id = match user_id { Some(id) => id, None => return };
    let title       = data["title"].as_str().map(str::to_string);
    let description = data["description"].as_str().unwrap_or("").trim().to_string();

    match store.create_chat(user_id, title.as_deref(), &description).await {
        Ok(chat) => {
            let _ = store.add_chat_member(chat.id, user_id).await;

            if let Some(tx) = bcast_tx {
                registry.join(chat.id, SessionEntry {
                    session_id,
                    user_id,
                    broadcast_tx: tx,
                }).await;
            }

            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "chat_created", "chat": chat })).await;
        }
        Err(e) => {
            log::error!("chat_create db error: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

