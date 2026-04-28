use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};
use rocket::get;
use rocket::State;
use rocket::futures::{SinkExt, StreamExt};
use rocket_ws::{Channel, Message, WebSocket};
use serde_json::{Value, json};
use super::session::{Session, SessionState};
use super::router::EventRouter;
use super::registry::{SessionRegistry, SessionEntry};
use super::handlers;
use crate::hnts::HntsState;
use crate::db::Store;
use crate::secure;
use crate::upload::UploadTokenStore;
use crate::admin::metric;

enum StateCtx {
    Entrypoint,
    LongToken(String),
    Authenticated(String),
    PrivateOnly(String),
}

fn state_ctx(sess: &Session) -> StateCtx {
    match &sess.state {
        SessionState::Entrypoint                        => StateCtx::Entrypoint,
        SessionState::LongToken    { private_vtns, .. } => StateCtx::LongToken(private_vtns.clone()),
        SessionState::Authenticated { priv_at, .. }     => StateCtx::Authenticated(priv_at.clone()),
        SessionState::PrivateOnly(p)                    => StateCtx::PrivateOnly(p.clone()),
    }
}

fn message_bytes(msg: Message) -> Option<Vec<u8>> {
    match msg {
        Message::Binary(b) => Some(b),
        Message::Text(t)   => Some(t.into_bytes()),
        _                  => None,
    }
}

fn decrypt_and_parse(key: &str, data: &[u8]) -> Option<(String, Value)> {
    let plain = secure::decrypt(key, data)?;
    let json: Value = serde_json::from_slice(&plain).ok()?;
    let event = json.get("event")?.as_str()?.to_string();
    Some((event, json))
}

async fn handle_token_refresh(sess: &mut Session) {
    let action = match &sess.state {
        SessionState::LongToken    { private_vtns, .. } => Some((false, private_vtns.clone())),
        SessionState::Authenticated { priv_at, .. }     => Some((true,  priv_at.clone())),
        _ => None,
    };
    let Some((is_auth, old_key)) = action else { return };
    let new_pub  = secure::get_token(16);
    let new_priv = secure::get_token(32);

    if is_auth {
        let enc = secure::encrypt(&old_key, json!({
            "event": "token_refresh", "pub_at": new_pub, "priv_at": new_priv,
        }).to_string().as_bytes());
        let _ = sess.send_binary(enc).await;
        sess.state = SessionState::Authenticated {
            pub_at: new_pub, priv_at: new_priv, issued_at: Instant::now(),
        };
    } else {
        let enc = secure::encrypt(&old_key, json!({
            "event": "token_refresh", "public_vtns": new_pub, "private_vtns": new_priv,
        }).to_string().as_bytes());
        let _ = sess.send_binary(enc).await;
        sess.state = SessionState::LongToken {
            public_vtns: new_pub, private_vtns: new_priv, issued_at: Instant::now(),
        };
    }
}


#[get("/ws/<pub_vtns>")]
pub fn ws(
    ws:           WebSocket,
    pub_vtns:     String,
    hnts:         &State<HntsState>,
    store:        &State<Arc<Store>>,
    registry:     &State<SessionRegistry>,
    upload_store: &State<Arc<UploadTokenStore>>,
    srv_state:    &State<metric::ServerState>,
) -> Channel<'static> {
    let hnts         = hnts.inner().clone();
    let store        = Arc::clone(store.inner());
    let registry     = registry.inner().clone();
    let upload_store = Arc::clone(upload_store.inner());
    let srv_state    = srv_state.inner().clone();

    ws.channel(move |stream| Box::pin(async move {
        srv_state.on_connect().await;
        let (mut sink, mut source) = stream.split();
        let (tx, mut rx)           = mpsc::channel::<Message>(32);

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sink.send(msg).await.is_err() { break; }
            }
        });

        let session = Arc::new(Mutex::new(Session::new(tx, pub_vtns)));

        let (bcast_tx, mut bcast_rx) = mpsc::channel::<Value>(64);
        {
            let mut s      = session.lock().await;
            s.store        = Some(Arc::clone(&store));
            s.broadcast_tx = Some(bcast_tx);
        }
        let bcast_sess = Arc::clone(&session);
        tokio::spawn(async move {
            while let Some(msg) = bcast_rx.recv().await {
                let s = bcast_sess.lock().await;
                s.send_encrypted(&msg).await;
            }
        });

        let reg = Arc::new(registry.clone());
        let mut router = EventRouter::new();
        {
            let r = Arc::clone(&reg);
            router.on("chat_join", move |sess, data| {
                let r = Arc::clone(&r);
                async move { handlers::chat_join(sess, data, r).await }
            });
        }
        {
            let r  = Arc::clone(&reg);
            let ss = srv_state.clone();
            router.on("message_create", move |sess, data| {
                let r  = Arc::clone(&r);
                let ss = ss.clone();
                async move { handlers::message_create(sess, data, r, ss).await }
            });
        }
        router.on("message_list", |sess, data| async move {
            handlers::message_list(sess, data).await
        });
        {
            let us = Arc::clone(&upload_store);
            router.on("get_upload_token", move |sess, data| {
                let us = Arc::clone(&us);
                async move { handlers::get_upload_token(sess, data, us).await }
            });
        }
        router.on("post_create", |sess, data| async move {
            handlers::post_create(sess, data).await
        });
        router.on("chat_list", |sess, data| async move {
            handlers::chat_list(sess, data).await
        });
        router.on("user_posts", |sess, data| async move {
            handlers::user_posts(sess, data).await
        });

        router.on("profile_get", |sess, data| async move {
            handlers::profile_get(sess, data).await
        });
        router.on("profile_update", |sess, data| async move {
            handlers::profile_update(sess, data).await
        });

        {
            let r = Arc::clone(&reg);
            router.on("roles_update", move |sess, data| {
                let r = Arc::clone(&r);
                async move { handlers::roles_update(sess, data, r).await }
            });
        }

        {
            let ss = srv_state.clone();
            router.on("terminal_cmd", move |sess, data| {
                let ss = ss.clone();
                async move { handlers::terminal_cmd(sess, data, ss).await }
            });
        }

        let router = Arc::new(router);

        while let Some(result) = source.next().await {
            let raw = match result { Ok(m) => m, Err(_) => break };
            let raw = match message_bytes(raw) { Some(b) => b, None => continue };

            let ctx = {
                let mut s = session.lock().await;
                if s.tokens_expired() { handle_token_refresh(&mut s).await; }
                s.last_activity = Instant::now();
                state_ctx(&s)
            };

            match ctx {
                StateCtx::Entrypoint => {
                    let Some(plain) = hnts.try_decrypt(&raw).await else {
                        let s = session.lock().await;
                        let _ = s.send_json(&json!({ "event": "error", "code": "decrypt_failed" })).await;
                        continue;
                    };
                    let Ok(val) = serde_json::from_slice::<Value>(&plain) else { continue };
                    if val["event"].as_str() != Some("auth") { continue; }

                    match (val["pub_vtns"].as_str(), val["priv_vtns"].as_str()) {
                        (Some(pv), Some(pvt)) => {
                            let mut s = session.lock().await;
                            if pv != s.pub_vtns {
                                let _ = s.send_json(&json!({ "event": "error", "code": "vtns_mismatch" })).await;
                                continue;
                            }
                            s.state = SessionState::LongToken {
                                public_vtns:  pv.to_string(),
                                    private_vtns: pvt.to_string(),
                                        issued_at:    Instant::now(),
                            };
                            let _ = s.send_json(&json!({ "event": "auth_ok" })).await;
                        }
                        _ => continue,
                    }
                }

                StateCtx::LongToken(priv_vtns) => {
                    let Some((event, payload)) = decrypt_and_parse(&priv_vtns, &raw) else {
                        let s = session.lock().await;
                        let _ = s.send_json(&json!({ "event": "error", "code": "decrypt_failed" })).await;
                        continue;
                    };

                    match event.as_str() {
                        "login" => {
                            let (username, password, session_id, bcast_tx) = {
                                let s = session.lock().await;
                                (
                                    payload["username"].as_str().unwrap_or("").to_string(),
                                 payload["password"].as_str().unwrap_or("").to_string(),
                                 s.id.clone(),
                                 s.broadcast_tx.clone(),
                                )
                            };

                            let login_result = async {
                                let user = store.get_user_by_name(&username).await
                                .map_err(|e| { error!("login get_user_by_name: {e}"); "db_error" })?
                                .ok_or("user_not_found")?;
                                if !secure::verify_password(&password, &user.pass) {
                                    return Err("wrong_password");
                                }
                                Ok(user)
                            }.await;

                            match login_result {
                                Ok(user) => {
                                    let pub_at  = secure::get_token(16);
                                    let priv_at = secure::get_token(32);

                                    let roles = store.get_user_roles(user.id).await.unwrap_or_default();
                                    let role_names: Vec<String>  = roles.iter().map(|r| r.name.clone()).collect();
                                    let permissions: Vec<i32>    = crate::db::roles::resolve_permissions(&roles);

                                    if let (Some(tx), Ok(chats)) = (&bcast_tx, store.get_user_chats(user.id).await) {
                                        for chat in chats {
                                            registry.join(chat.id, SessionEntry {
                                                session_id:   session_id.clone(),
                                                          user_id:      user.id,
                                                          broadcast_tx: tx.clone(),
                                            }).await;
                                        }
                                    }
                                    registry.register_user_session(user.id, Arc::clone(&session)).await;

                                    let enc = secure::encrypt(&priv_vtns, json!({
                                        "event":       "login_ok",
                                        "pub_at":      pub_at,
                                        "priv_at":     priv_at,
                                        "user_id":     user.id,
                                        "username":    user.name,
                                        "roles":       role_names,
                                        "permissions": permissions,
                                    }).to_string().as_bytes());

                                    let mut s = session.lock().await;
                                    let _ = s.send_binary(enc).await;
                                    s.user_id     = Some(user.id);
                                    s.permissions = permissions;
                                    s.state       = SessionState::Authenticated {
                                        pub_at, priv_at, issued_at: Instant::now(),
                                    };
                                }
                                Err(code) => {
                                    let s = session.lock().await;
                                    let _ = s.send_json(&json!({ "event": "login_failed", "code": code })).await;
                                }
                            }
                        }

                        "register" => {
                            let (username, password, session_id, bcast_tx) = {
                                let s = session.lock().await;
                                (
                                    payload["username"].as_str().unwrap_or("").to_string(),
                                 payload["password"].as_str().unwrap_or("").to_string(),
                                 s.id.clone(),
                                 s.broadcast_tx.clone(),
                                )
                            };

                            let register_result = async {
                                if username.len() < 3 || username.len() > 32 {
                                    return Err("invalid_username");
                                }
                                if password.len() < 6 {
                                    return Err("password_too_short");
                                }
                                if store.get_user_by_name(&username).await
                                    .map_err(|_| "db_error")?
                                    .is_some()
                                    {
                                        return Err("username_taken");
                                    }
                                    let hashed = tokio::task::spawn_blocking(move || secure::hash_password(&password))
                                    .await
                                    .map_err(|_| "hash_error")?
                                    .map_err(|_| "hash_error")?;
                                store.insert_user(&username, &hashed)
                                .await
                                .map_err(|_| "db_error")
                            }.await;

                            match register_result {
                                Ok(user) => {
                                    let pub_at  = secure::get_token(16);
                                    let priv_at = secure::get_token(32);

                                    let roles       = store.get_user_roles(user.id).await.unwrap_or_default();
                                    let role_names: Vec<String> = roles.iter().map(|r| r.name.clone()).collect();
                                    let permissions: Vec<i32>   = crate::db::roles::resolve_permissions(&roles);

                                    registry.register_user_session(user.id, Arc::clone(&session)).await;

                                    let enc = secure::encrypt(&priv_vtns, json!({
                                        "event":       "register_ok",
                                        "pub_at":      pub_at,
                                        "priv_at":     priv_at,
                                        "user_id":     user.id,
                                        "username":    user.name,
                                        "roles":       role_names,
                                        "permissions": permissions,
                                    }).to_string().as_bytes());

                                    let mut s = session.lock().await;
                                    let _ = s.send_binary(enc).await;
                                    s.user_id     = Some(user.id);
                                    s.permissions = permissions;
                                    s.state       = SessionState::Authenticated {
                                        pub_at, priv_at, issued_at: Instant::now(),
                                    };
                                }
                                Err(code) => {
                                    let s = session.lock().await;
                                    let _ = s.send_json(&json!({ "event": "register_failed", "code": code })).await;
                                }
                            }
                        }
                        _ => {}
                    }
                }

                StateCtx::Authenticated(priv_at) => {
                    match decrypt_and_parse(&priv_at, &raw) {
                        Some((event, payload)) => {
                            session.lock().await.is_authenticated = true;
                            srv_state.on_user_authenticated().await;
                            router.dispatch(&event, Arc::clone(&session), payload).await;
                        }
                        None => {
                            let mut s = session.lock().await;
                            s.state = SessionState::PrivateOnly(priv_at);
                            let _ = s.send_json(&json!({ "event": "reauth_required" })).await;
                        }
                    }
                }

                StateCtx::PrivateOnly(priv_at) => {
                    let expected = secure::token_hash(&priv_at);
                    let Ok(incoming) = serde_json::from_slice::<Value>(&raw) else { continue };
                    if incoming["event"].as_str() != Some("reauth") { continue; }
                    let provided = incoming["hash"].as_str().unwrap_or("");

                    let mut s = session.lock().await;
                    if provided == expected {
                        let new_pub  = secure::get_token(16);
                        let new_priv = secure::get_token(32);
                        let enc = secure::encrypt(&priv_at, json!({
                            "event": "reauth_ok", "pub_at": new_pub,
                        }).to_string().as_bytes());
                        let _ = s.send_binary(enc).await;
                        s.state = SessionState::Authenticated {
                            pub_at: new_pub, priv_at: new_priv, issued_at: Instant::now(),
                        };
                    } else {
                        s.state = SessionState::Entrypoint;
                        let _ = s.send_json(&json!({ "event": "reauth_failed" })).await;
                    }
                }
            }
        }

        { registry.leave(&session.lock().await.id).await; }
        let is_auth = session.lock().await.is_authenticated;
        registry.leave(&session.lock().await.id).await;
        srv_state.on_disconnect().await;
        if is_auth { srv_state.on_user_left().await; }

        Ok(())
    }))
}
