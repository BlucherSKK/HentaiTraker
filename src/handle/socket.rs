use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};
use rocket::get;
use rocket::State;
use rocket::futures::{SinkExt, StreamExt};
use rocket_ws::{Channel, Message, WebSocket};
use serde_json::Value;
use super::session::{Session, SessionState};
use super::router::EventRouter;
use crate::hnts::HntsState;
use crate::secure;

// ─── Контекст состояния ───────────────────────────────────────────────────────
//
// Снимок нужных данных из Session, извлекается под коротким локом.
// Позволяет отпустить Mutex до дешифрования.

enum StateCtx {
    Entrypoint,
    LongToken(String),     // private_vtns
    Authenticated(String), // priv_at
    PrivateOnly(String),   // raw priv_at; хеш вычисляется по месту
}

fn state_ctx(sess: &Session) -> StateCtx {
    match &sess.state {
        SessionState::Entrypoint => StateCtx::Entrypoint,
        SessionState::LongToken { private_vtns, .. } => StateCtx::LongToken(private_vtns.clone()),
        SessionState::Authenticated { priv_at, .. } => StateCtx::Authenticated(priv_at.clone()),
        SessionState::PrivateOnly(p) => StateCtx::PrivateOnly(p.clone()),
    }
}

// ─── Вспомогательные функции ──────────────────────────────────────────────────

/// Извлекает байты из WebSocket-сообщения.
/// Text конвертируется в байты; управляющие фреймы (Ping/Pong/Close) → None.
fn message_bytes(msg: Message) -> Option<Vec<u8>> {
    match msg {
        Message::Binary(b) => Some(b),
        Message::Text(t) => Some(t.into_bytes()),
        _ => None,
    }
}

/// Дешифрует данные ключом key, парсит JSON и возвращает (event, payload).
/// Поле "event" обязательно должно быть строковым полем объекта.
fn decrypt_and_parse(key: &str, data: &[u8]) -> Option<(String, Value)> {
    let plain = secure::decrypt(key, data)?;
    let json: Value = serde_json::from_slice(&plain).ok()?;
    let event = json.get("event")?.as_str()?.to_string();
    Some((event, json))
}

/// Генерирует новую пару токенов, отправляет зашифрованное уведомление клиенту
/// и обновляет состояние сессии. Вызывается только если `sess.tokens_expired()`.
async fn handle_token_refresh(sess: &mut Session) {
    let action = match &sess.state {
        SessionState::LongToken { private_vtns, .. } => Some((false, private_vtns.clone())),
        SessionState::Authenticated { priv_at, .. }  => Some((true,  priv_at.clone())),
        _ => None,
    };
    let Some((is_auth, old_key)) = action else { return };

    let new_pub  = secure::get_token(16);
    let new_priv = secure::get_token(32);

    if is_auth {
        let msg = serde_json::json!({
            "event":   "token_refresh",
            "pub_at":  new_pub,
            "priv_at": new_priv,
        });
        let enc = secure::encrypt(&old_key, msg.to_string().as_bytes());
        let _ = sess.send_binary(enc).await;
        sess.state = SessionState::Authenticated {
            pub_at:    new_pub,
            priv_at:   new_priv,
            issued_at: Instant::now(),
        };
    } else {
        let msg = serde_json::json!({
            "event":       "token_refresh",
            "public_vtns": new_pub,
            "private_vtns": new_priv,
        });
        let enc = secure::encrypt(&old_key, msg.to_string().as_bytes());
        let _ = sess.send_binary(enc).await;
        sess.state = SessionState::LongToken {
            public_vtns:  new_pub,
                private_vtns: new_priv,
                    issued_at:    Instant::now(),
        };
    }
}

// ─── WebSocket route ──────────────────────────────────────────────────────────

/// Точка входа WebSocket-сессии.
///
/// ## Протокол состояний
///
/// ```
/// Entrypoint
///   ← binary( encrypt(hnts_token, { event:"auth", pub_vtns, priv_vtns }) )
///   → send json { event:"auth_ok" }
///   → LongToken
///
/// LongToken
///   ← binary( encrypt(priv_vtns, { event:"login", username, password }) )
///   → send binary( encrypt(priv_vtns, { event:"login_ok", pub_at, priv_at }) )
///   → Authenticated
///
/// Authenticated
///   ← binary( encrypt(priv_at, { event:"<name>", ... }) )
///   → dispatch(EventRouter) по полю "event"
///   → если дешифровка провалилась → PrivateOnly
///
/// PrivateOnly
///   ← plaintext json { event:"reauth", hash:"<sha256_hex(priv_at)>" }
///   → send binary( encrypt(priv_at, { event:"reauth_ok", pub_at }) ) + Authenticated
///   → или json { event:"reauth_failed" } + Entrypoint
/// ```
///
/// Route: `GET /api/hnts/ws/<pub_vtns>`
#[get("/ws/<pub_vtns>")]
pub fn ws(ws: WebSocket, pub_vtns: String, hnts: &State<HntsState>) -> Channel<'static> {
    let hnts = hnts.inner().clone();

    ws.channel(move |stream| {
        Box::pin(async move {
            let (mut sink, mut source) = stream.split();
            let (tx, mut rx) = mpsc::channel::<Message>(32);

            // Отдельная задача-писатель: сериализует отправку в сокет.
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if sink.send(msg).await.is_err() {
                        break;
                    }
                }
            });

            let session = Arc::new(Mutex::new(Session::new(tx, pub_vtns)));

            // ── Регистрация обработчиков приложения ───────────────────────────
            let mut router = EventRouter::new();
            // router.on("message_create", |sess, data| async move { ... });
            // router.on("message_list",   |sess, data| async move { ... });
            // router.on("chat_create",    |sess, data| async move { ... });
            // router.on("chat_list",      |sess, data| async move { ... });
            // router.on("post_create",    |sess, data| async move { ... });
            let router = Arc::new(router);
            // ─────────────────────────────────────────────────────────────────

            while let Some(result) = source.next().await {
                let raw_msg = match result {
                    Ok(m) => m,
                 Err(_) => break,
                };
                let raw_bytes = match message_bytes(raw_msg) {
                    Some(b) => b,
                 None => continue,
                };

                // Единственный лок на итерацию: refresh + last_activity + snapshot.
                let ctx = {
                    let mut sess = session.lock().await;
                    if sess.tokens_expired() {
                        handle_token_refresh(&mut sess).await;
                    }
                    sess.last_activity = Instant::now();
                    state_ctx(&sess)
                };

                match ctx {
                    // ── Entrypoint ────────────────────────────────────────────
                    StateCtx::Entrypoint => {
                        let plain = match hnts.try_decrypt(&raw_bytes).await {
                            Some(p) => p,
                 None => {
                     let sess = session.lock().await;
                     let _ = sess.send_json(&serde_json::json!({
                         "event": "error",
                         "code":  "decrypt_failed",
                     })).await;
                     continue;
                 }
                        };
                        let json: Value = match serde_json::from_slice(&plain) {
                            Ok(v) => v,
                 Err(_) => continue,
                        };
                        if json.get("event").and_then(Value::as_str) != Some("auth") {
                            continue;
                        }
                        let pub_v  = json.get("pub_vtns").and_then(Value::as_str).map(str::to_string);
                        let priv_v = json.get("priv_vtns").and_then(Value::as_str).map(str::to_string);
                        match (pub_v, priv_v) {
                            (Some(pub_v), Some(priv_v)) => {
                                let mut sess = session.lock().await;
                                if pub_v != sess.pub_vtns {
                                    let _ = sess.send_json(&serde_json::json!({
                                        "event": "error",
                                        "code":  "vtns_mismatch",
                                    })).await;
                                    continue;
                                }
                                sess.state = SessionState::LongToken {
                                    public_vtns:  pub_v,
                                        private_vtns: priv_v,
                                            issued_at:    Instant::now(),
                                };
                                let _ = sess.send_json(&serde_json::json!({
                                    "event": "auth_ok",
                                })).await;
                            }
                            _ => continue,
                        }
                    }

                    // ── LongToken ─────────────────────────────────────────────
                    StateCtx::LongToken(private_vtns) => {
                        let (event, payload) = match decrypt_and_parse(&private_vtns, &raw_bytes) {
                            Some(v) => v,
                 None => {
                     let sess = session.lock().await;
                     let _ = sess.send_json(&serde_json::json!({
                         "event": "error",
                         "code":  "decrypt_failed",
                     })).await;
                     continue;
                 }
                        };
                        if event != "login" {
                            continue;
                        }

                        // TODO: проверить payload["username"] / payload["password"] через sess.store
                        let _ = payload;

                        let pub_at  = secure::get_token(16);
                        let priv_at = secure::get_token(32);
                        let response = serde_json::json!({
                            "event":   "login_ok",
                            "pub_at":  pub_at,
                            "priv_at": priv_at,
                        });
                        let enc = secure::encrypt(&private_vtns, response.to_string().as_bytes());
                        let mut sess = session.lock().await;
                        let _ = sess.send_binary(enc).await;
                        sess.state = SessionState::Authenticated {
                            pub_at,
                            priv_at,
                            issued_at: Instant::now(),
                        };
                    }

                    // ── Authenticated ─────────────────────────────────────────
                    StateCtx::Authenticated(priv_at) => {
                        match decrypt_and_parse(&priv_at, &raw_bytes) {
                            Some((event, payload)) => {
                                router.dispatch(&event, Arc::clone(&session), payload).await;
                            }
                            None => {
                                // Дешифровка провалилась → pub_at скомпрометирован.
                                let mut sess = session.lock().await;
                                sess.state = SessionState::PrivateOnly(priv_at);
                                let _ = sess.send_json(&serde_json::json!({
                                    "event": "reauth_required",
                                })).await;
                            }
                        }
                    }

                    // ── PrivateOnly ───────────────────────────────────────────
                    StateCtx::PrivateOnly(priv_at) => {
                        let expected_hash = secure::token_hash(&priv_at);
                        let incoming: Value = match serde_json::from_slice(&raw_bytes) {
                            Ok(v) => v,
                 Err(_) => continue,
                        };
                        if incoming.get("event").and_then(Value::as_str) != Some("reauth") {
                            continue;
                        }
                        let provided = incoming.get("hash").and_then(Value::as_str).unwrap_or("");

                        let mut sess = session.lock().await;
                        if provided == expected_hash {
                            let new_pub  = secure::get_token(16);
                            let new_priv = secure::get_token(32);
                            let response = serde_json::json!({
                                "event":  "reauth_ok",
                                "pub_at": new_pub,
                            });
                            let enc = secure::encrypt(&priv_at, response.to_string().as_bytes());
                            let _ = sess.send_binary(enc).await;
                            sess.state = SessionState::Authenticated {
                                pub_at:    new_pub,
                                priv_at:   new_priv,
                                issued_at: Instant::now(),
                            };
                        } else {
                            sess.state = SessionState::Entrypoint;
                            let _ = sess.send_json(&serde_json::json!({
                                "event": "reauth_failed",
                            })).await;
                        }
                    }
                }
            }

            Ok(())
        })
    })
}
