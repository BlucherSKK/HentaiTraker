use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::{Value, json};
use super::session::Session;
use super::registry::{SessionRegistry, SessionEntry};
use crate::upload::UploadTokenStore;
use crate::admin;
use crate::admin::metric;
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


// ─── profile_get ─────────────────────────────────────────────────────────────

/// Получение профиля пользователя.
///
/// Payload:  `{ user_id?: i32 }` (по умолчанию — свой профиль)
/// Ответ OK: `{ event: "profile_ok", id, name, avatar, tags, roles }`
pub async fn profile_get(session: Arc<Mutex<Session>>, data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };
    let store = match store { Some(s) => s, None => return };
    let requester_id = match user_id {
        Some(id) => id,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "unauthenticated" })).await;
            return;
        }
    };

    let target_id = data["user_id"].as_i64().map(|id| id as i32).unwrap_or(requester_id);

    match store.get_user(target_id).await {
        Ok(Some(user)) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({
                "event":  "profile_ok",
                "id":     user.id,
                "name":   user.name,
                "avatar": user.avatar,
                "tags":   user.tags,
                "roles":  user.roles,
            })).await;
        }
        Ok(None) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "user_not_found" })).await;
        }
        Err(e) => {
            error!("profile_get: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

// ─── profile_update ──────────────────────────────────────────────────────────

/// Обновление профиля пользователя.
///
/// Payload:  `{ user_id?: i32, name?: string, avatar?: string, tags?: string, roles?: string }`
/// Ответ OK: `{ event: "profile_updated", id, name, avatar, tags, roles }`
/// Ошибки:   `{ event: "error", code: "unauthorized" | "user_not_found" | "db_error" }`
pub async fn profile_update(session: Arc<Mutex<Session>>, data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };
    let store = match store { Some(s) => s, None => return };
    let modifier_id = match user_id {
        Some(id) => id,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "unauthenticated" })).await;
            return;
        }
    };

    let target_id = data["user_id"].as_i64().map(|id| id as i32).unwrap_or(modifier_id);
    let name   = data["name"].as_str();
    let avatar = data["avatar"].as_str();
    let tags   = data["tags"].as_str();
    let roles  = data["roles"].as_str();

    match store.update_user(target_id, modifier_id, name, None, avatar, tags, roles).await {
        Ok(Some(user)) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({
                "event":  "profile_updated",
                "id":     user.id,
                "name":   user.name,
                "avatar": user.avatar,
                "tags":   user.tags,
                "roles":  user.roles,
            })).await;
        }
        Ok(None) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "unauthorized" })).await;
        }
        Err(e) => {
            error!("profile_update: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
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
    srv_state: metric::ServerState,
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

    // files — JSON-массив URL строк: ["/api/files/xxx.jpg", ...]
    let files = data["files"].as_array()
    .map(|arr| serde_json::to_string(arr).unwrap_or_default());

    match store.send_message(chat_id, user_id, &content, files.as_deref()).await {
        Ok(msg) => {
            srv_state.on_message_sent().await;
            let broadcast = json!({
                "event":     "new_message",
                "id":        msg.id,
                "chat_id":   msg.chat_id,
                "author_id": msg.author_id,
                "content":   msg.content,
                "files":     msg.files,   // теперь передаём клиентам
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

// ─── get_upload_token ─────────────────────────────────────────────────────────

/// Payload: `{}`
/// Ответ:   `{ event: "upload_token", token: "..." }` — одноразовый, TTL 5 минут
pub async fn get_upload_token(
    session:      Arc<Mutex<Session>>,
    _data:        Value,
    upload_store: Arc<UploadTokenStore>,
) {
    let user_id = {
        let s = session.lock().await;
        s.user_id
    };
    let user_id = match user_id { Some(id) => id, None => return };

    let token = upload_store.create_token(user_id).await;
    let s = session.lock().await;
    s.send_encrypted(&json!({ "event": "upload_token", "token": token })).await;
}

// ─── terminal_cmd ─────────────────────────────────────────────────────────────

/// Payload: `{ input: string }`
pub async fn terminal_cmd(session: Arc<Mutex<Session>>, data: Value, srv_state: metric::ServerState) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };
    let store   = match store   { Some(s) => s, None => return };
    let user_id = match user_id { Some(id) => id, None => return };

    let roles = match store.get_user(user_id).await {
        Ok(Some(u)) => u.roles.unwrap_or_default(),
        _ => return,
    };
    if !roles.split(',').any(|r| r.trim() == "admin") {
        let s = session.lock().await;
        s.send_encrypted(&json!({ "event": "error", "code": "forbidden" })).await;
        return;
    }

    let input = match data["input"].as_str() {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return,
    };

    let output = admin::hnts_shell_exec(&input, srv_state.snapshot().await.format());
    let s = session.lock().await;
    s.send_encrypted(&json!({ "event": "terminal_output", "output": output })).await;
}


