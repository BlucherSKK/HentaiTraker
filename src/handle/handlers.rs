use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::{Value, json};
use super::session::Session;
use super::registry::{SessionRegistry, SessionEntry};
use crate::lfs::UploadTokenStore;
use crate::admin;
use crate::admin::metric;
use crate::db::roles::{resolve_permissions, role_names, Permission};
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


// ----- profile_get -----

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

    let user = match store.get_user(target_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "user_not_found" })).await;
            return;
        }
        Err(e) => {
            error!("profile_get: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
            return;
        }
    };

    let roles = match store.get_user_roles(target_id).await {
        Ok(r) => r,
        Err(e) => {
            error!("profile_get roles: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
            return;
        }
    };

    let permissions = resolve_permissions(&roles);
    let s = session.lock().await;
    s.send_encrypted(&json!({
        "event":       "profile_ok",
        "id":          user.id,
        "name":        user.name,
        "avatar":      user.avatar,
        "tags":        user.tags,
        "roles":       role_names(&roles),
                            "permissions": permissions,
    })).await;
}

// ----- profile_update -----

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

    let user = match store.update_user(target_id, modifier_id, name, None, avatar, tags).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "unauthorized" })).await;
            return;
        }
        Err(e) => {
            error!("profile_update: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
            return;
        }
    };

    let roles = match store.get_user_roles(target_id).await {
        Ok(r) => r,
        Err(e) => {
            error!("profile_update roles: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
            return;
        }
    };

    let permissions = resolve_permissions(&roles);
    let s = session.lock().await;
    s.send_encrypted(&json!({
        "event":       "profile_updated",
        "id":          user.id,
        "name":        user.name,
        "avatar":      user.avatar,
        "tags":        user.tags,
        "roles":       role_names(&roles),
                            "permissions": permissions,
    })).await;
}

// ----- roles_update -----
// Payload:  { user_id: i32, role_ids: i32[] }
// Ответ OK: { event: "roles_updated", user_id, roles: string[], permissions: i32[] }
// Ошибки:   { event: "error", code: "unauthenticated" | "unauthorized" | "db_error" }

/// ----- roles_update -----

pub async fn roles_update(
    session:  Arc<Mutex<Session>>,
    data:     Value,
    registry: Arc<SessionRegistry>,
) {
    let (store, modifier_id, can_manage) = {
        let s = session.lock().await;
        let can = s.has_permission(Permission::ManageRoles.as_i32());
        (s.store.clone(), s.user_id, can)
    };

    let store = match store { Some(s) => s, None => return };

    let modifier_id = match modifier_id {
        Some(id) => id,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "unauthenticated" })).await;
            return;
        }
    };

    if !can_manage {
        let s = session.lock().await;
        s.send_encrypted(&json!({ "event": "error", "code": "unauthorized" })).await;
        return;
    }

    let target_id = match data["user_id"].as_i64() {
        Some(id) => id as i32,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "missing_user_id" })).await;
            return;
        }
    };

    let role_ids: Vec<i32> = match data["role_ids"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect(),
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "missing_role_ids" })).await;
            return;
        }
    };

    let roles = match store.db.set_user_roles(target_id, &role_ids).await {
        Ok(_) => match store.get_user_roles(target_id).await {
            Ok(r) => r,
            Err(e) => {
                error!("roles_update get_user_roles: {e}");
                let s = session.lock().await;
                s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
                return;
            }
        },
        Err(e) => {
            error!("roles_update set_user_roles: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
            return;
        }
    };

    let names       = role_names(&roles);
    let permissions = resolve_permissions(&roles);

    registry.notify_roles_updated(target_id, &names, &permissions).await;

    let s = session.lock().await;
    s.send_encrypted(&json!({
        "event":       "roles_update_ok",
        "user_id":     target_id,
        "roles":       names,
        "permissions": permissions,
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


// ----- post_create -----


fn validate_tags(raw: &str) -> String {
    let mut tags: Vec<&str> = raw
    .split(',')
    .map(str::trim)
    .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
    .collect();

    if !tags.contains(&"any") {
        tags.insert(0, "any");
    }

    tags.join(",")
}

// ----- utils -----

fn strip_nulls(s: &str) -> String {
    s.replace('\0', "")
}

/// Payload: `{ title?: string, content: string, files?: string }`
/// Ответ:   `{ event: "post_created", post: { id, title, content, files, author_id, time } }`
pub async fn post_create(session: Arc<Mutex<Session>>, data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };
    let store   = match store   { Some(s) => s, None => return };
    let user_id = match user_id { Some(id) => id, None => return };

    let has_posting = match store.user_has_permission(user_id, Permission::Posting).await {
        Ok(v)  => v,
        Err(e) => {
            error!("post_create permission check: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
            return;
        }
    };

    if !has_posting {
        let s = session.lock().await;
        s.send_encrypted(&json!({ "event": "error", "code": "forbidden" })).await;
        return;
    }

    let title_s   = data["title"].as_str()
    .filter(|s| !s.is_empty())
    .map(strip_nulls);
    let content_s = strip_nulls(data["content"].as_str().unwrap_or("").trim());
    let tags_s    = strip_nulls(&validate_tags(data["tags"].as_str().unwrap_or("")));

    if content_s.is_empty() {
        let s = session.lock().await;
        s.send_encrypted(&json!({ "event": "error", "code": "empty_content" })).await;
        return;
    }
    let files_s = data["files"].as_str()
    .filter(|s| !s.is_empty())
    .map(strip_nulls);

    match store.create_post(user_id, title_s.as_deref(), &content_s, files_s.as_deref(), Some(&tags_s)).await {
        Ok(post) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({
                "event": "post_created",
                "post": {
                    "id":        post.id,
                    "title":     post.title,
                    "content":   post.content,
                    "files":     post.files,
                    "tags":      post.tags,
                    "author_id": post.author_id,
                    "time":      post.time.to_string(),
                }
            })).await;
        }
        Err(e) => {
            error!("post_create: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}


pub async fn user_posts(session: Arc<Mutex<Session>>, data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };
    let store   = match store   { Some(s) => s, None => return };
    let user_id = match user_id { Some(id) => id, None => return };

    let limit = data["limit"].as_i64().unwrap_or(20).clamp(1, 100);

    match store.get_posts_by_author(user_id, limit as i32).await {
        Ok(posts) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "user_posts", "posts": posts })).await;
        }
        Err(e) => {
            error!("user_posts: {e}");
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
    let perm = {
        let s = session.lock().await;
        s.permissions.clone()
    };
    let accept  = perm.contains(&(Permission::Terminal as i32));

    if !accept {
        let s = session.lock().await;
        s.send_encrypted(&json!({ "event": "error", "code": "forbidden" })).await;
        return;
    }

    let input = match data["input"].as_str() {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return,
    };

    let output = admin::hnts_shell_exec(&input, srv_state.snapshot().await.format());

    let final_output = if let Some(id_str) = output.strip_prefix("news:set:") {
        match id_str.trim().parse::<i32>() {
            Ok(post_id) => {
                srv_state.set_sidebar_post_id(post_id).await;
                format!("sidebar привязана к посту #{}", post_id)
            }
            Err(_) => "ошибка парсинга id".into(),
        }
    } else {
        output
    };

    let s = session.lock().await;
    s.send_encrypted(&json!({ "event": "terminal_output", "output": final_output })).await;
}


// ----- settings_get -----

pub async fn settings_get(session: Arc<Mutex<Session>>, _data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };
    let store   = match store   { Some(s) => s, None => return };
    let user_id = match user_id {
        Some(id) => id,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "unauthenticated" })).await;
            return;
        }
    };

    match store.get_settings(user_id).await {
        Ok(settings) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "settings_ok", "settings": settings })).await;
        }
        Err(e) => {
            error!("settings_get: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

// ----- settings_update -----

pub async fn settings_update(session: Arc<Mutex<Session>>, data: Value) {
    let (store, user_id) = {
        let s = session.lock().await;
        (s.store.clone(), s.user_id)
    };
    let store   = match store   { Some(s) => s, None => return };
    let user_id = match user_id {
        Some(id) => id,
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "unauthenticated" })).await;
            return;
        }
    };

    let settings_str = match data["settings"].as_str() {
        Some(s) => s.to_string(),
        None => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "missing_settings" })).await;
            return;
        }
    };

    match store.set_settings(user_id, &settings_str).await {
        Ok(_) => {
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "settings_saved", "settings": settings_str })).await;
        }
        Err(e) => {
            error!("settings_update: {e}");
            let s = session.lock().await;
            s.send_encrypted(&json!({ "event": "error", "code": "db_error" })).await;
        }
    }
}

