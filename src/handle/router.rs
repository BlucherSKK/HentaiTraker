use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::Value;
use super::session::Session;

/// Тип обработчика события WebSocket-сессии.
/// Сигнатура: `async fn(Arc<Mutex<Session>>, Value)`
pub type HandlerFn = Box<
dyn Fn(Arc<Mutex<Session>>, Value) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
+ Send
+ Sync
+ 'static,
>;


/// Маршрутизатор событий аутентифицированной сессии.
///
/// # Пример
/// ```rust
/// router
///     .on("message_create", |sess, data| async move {
///         let guard = sess.lock().await;
///         let chat_id = data["chat_id"].as_i64().unwrap_or(0);
///         let content = data["content"].as_str().unwrap_or("");
///         // guard.store.as_ref().unwrap()
///         //     .send_message(chat_id as i32, guard.user_id.unwrap(), content).await
///     })
///     .on("chat_list", |sess, _data| async move {
///         // ...
///     });
/// ```
pub struct EventRouter {
    handlers: HashMap<String, HandlerFn>,
}

impl EventRouter {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Регистрирует обработчик для события `event`. Возвращает `&mut Self` для chain.
    pub fn on<F, Fut>(&mut self, event: impl Into<String>, handler: F) -> &mut Self
    where
    F: Fn(Arc<Mutex<Session>>, Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    {
        self.handlers.insert(
            event.into(),
                             Box::new(move |sess, val| Box::pin(handler(sess, val))),
        );
        self
    }

    /// Вызывает обработчик для `event`. Нет-оп если обработчик не зарегистрирован.
    pub async fn dispatch(&self, event: &str, session: Arc<Mutex<Session>>, payload: Value) {
        if let Some(handler) = self.handlers.get(event) {
            handler(session, payload).await;
        }
    }
}
