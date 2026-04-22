use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::{Value, json};
use super::session::Session;
use super::registry::{SessionRegistry, SessionEntry};

// ─── chat_join ────────────────────────────────────────────────────────────────

/// Вступление пользователя в существующий чат.
///
/// Payload:  `{ chat_id: i32 }`
/// Ответ OK: `{ event: "chat_joined", chat: Chat, already_member: bool }`
/// Ошибки:   `{ event: "error", code: "chat_not_found" | "db_error" | ... }`
pub async fn chat_join(
    session:  Arc<Mutex<Session>>,
    data:     Value,
    registry: Arc<SessionRegistry>,
) {
    // Берём лок только для чтения данных — не держим во время DB-запроса
    let (store, user_id, session_id, bcast_tx) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id, s.id.clone(), s.broadcast_tx.clone())
    };

    let store = match store {
        Some(s) => s,
        None => return,
    };
    let user_id = match user_id {
        Some(id) => id,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "unauthenticated" })).await;
            return;
        }
    };
    let chat_id = match data["chat_id"].as_i64() {
        Some(id) => id as i32,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "missing_chat_id" })).await;
            return;
        }
    };

    // Проверяем, не состоит ли уже в чате (для флага already_member в ответе)
    let already = match store.db_is_member(chat_id, user_id).await {
        Ok(v)  => v,
        Err(e) => {
            error!("chat_join is_member check: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
            return;
        }
    };

    // join_chat: проверяет существование чата + idempotent INSERT
    let chat = match store.join_chat(chat_id, user_id).await {
        Ok(c)  => c,
        Err(crate::db::StoreError::NotFound) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "chat_not_found" })).await;
            return;
        }
        Err(e) => {
            error!("chat_join db: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
            return;
        }
    };

    // Подписываем сессию на broadcast этого чата
    if !already {
        if let Some(tx) = bcast_tx {
            registry.join(chat_id, SessionEntry {
                session_id:   session_id.clone(),
                          user_id,
                          broadcast_tx: tx,
            }).await;

            // Уведомляем других участников чата о новом члене
            registry.broadcast_to_chat(
                chat_id,
                json!({ "event": "member_joined", "chat_id": chat_id, "user_id": user_id }),
                                       Some(&session_id),
            ).await;
        }
    }

    // Отправляем подтверждение вступившему
    let s = session.lock().await;
    s.send_encrypted(&json!({
        "event":          "chat_joined",
        "chat":           chat,
        "already_member": already,
    })).await;
}

// ─── chat_list ───────────────────────────────────────────────────────────────

/// Payload: `{}`
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
            error!("chat_list: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

// ─── message_create ───────────────────────────────────────────────────────────

/// Payload: `{ chat_id: i32, content: string }`
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
    let chat_id = match data["chat_id"].as_i64() { Some(id) => id as i32, None => return };
    let content = data["content"].as_str().unwrap_or("").trim().to_string();
    if content.is_empty() { return; }

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
            registry.broadcast_to_chat(chat_id, broadcast.clone(), Some(&session_id)).await;
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "message_sent", "message": broadcast })).await;
        }
        Err(e) => {
            error!("message_create: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

// ─── message_list ────────────────────────────────────────────────────────────

/// Payload: `{ chat_id: i32, limit?: i64 }`
pub async fn message_list(session: Arc<Mutex<Session>>, data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };
    let store   = match store   { Some(s) => s, None => return };
    let _       = match user_id { Some(id) => id, None => return };
    let chat_id = match data["chat_id"].as_i64() { Some(id) => id as i32, None => return };
    let limit   = data["limit"].as_i64().unwrap_or(50).clamp(1, 100);

    match store.get_chat_messages(chat_id, limit).await {
        Ok(msgs) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({
                "event":    "message_list",
                "chat_id":  chat_id,
                "messages": msgs,
            })).await;
        }
        Err(e) => {
            error!("message_list: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}
