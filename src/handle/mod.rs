use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use rocket::get;
use rocket::State;
use rocket::futures::{SinkExt, StreamExt};
use rocket_ws::{Channel, Message, WebSocket};
use crate::hnts::HntsState;

const TOKEN_LIFETIME: Duration = Duration::from_secs(20 * 60);

pub enum SessionState {
    Entrypoint,
    LongToken {
        public_vtns: String,
            private_vtns: String,
                issued_at: Instant,
    },
    Authenticated {
        pub_at: String,
        priv_at: String,
        issued_at: Instant,
    },
    PrivateOnly(String),
}

pub struct Session {
    pub state: SessionState,
    pub tx: mpsc::Sender<Message>,
}

impl Session {
    /// Создаёт новую сессию в начальном состоянии Entrypoint.
    /// @param tx - отправитель канала для исходящих WebSocket-сообщений
    pub fn new(tx: mpsc::Sender<Message>) -> Self {
        Self {
            state: SessionState::Entrypoint,
            tx,
        }
    }

    /// Отправляет текстовое сообщение клиенту через WebSocket-канал сессии.
    /// @param msg - текстовое содержимое сообщения
    pub async fn send(&self, msg: impl Into<String>) -> Result<(), mpsc::error::SendError<Message>> {
        self.tx.send(Message::Text(msg.into())).await
    }

    /// Отправляет бинарное сообщение клиенту через WebSocket-канал сессии.
    /// @param data - бинарные данные для отправки
    pub async fn send_binary(&self, data: Vec<u8>) -> Result<(), mpsc::error::SendError<Message>> {
        self.tx.send(Message::Binary(data)).await
    }

    /// Передаёт входящее WebSocket-сообщение в пользовательский асинхронный обработчик.
    /// @param msg     - входящее сообщение от клиента
    /// @param handler - замыкание вида `|sess, msg| Box::pin(async move { ... })`,
    ///                  захватываемые значения должны реализовывать Send
    pub async fn on_message<'s, F>(&'s mut self, msg: Message, handler: F)
    where
    F: FnOnce(&'s mut Self, Message) -> Pin<Box<dyn Future<Output = ()> + Send + 's>>,
    {
        handler(self, msg).await;
    }

    /// Возвращает true если текущая пара токенов сессии истекла (~20 мин).
    /// Применяется как к неавторизованной паре ВТНС, так и к авторизованной ВПубАТ+ВПривАТ.
    pub fn tokens_expired(&self) -> bool {
        match &self.state {
            SessionState::LongToken { issued_at, .. } |
            SessionState::Authenticated { issued_at, .. } => issued_at.elapsed() >= TOKEN_LIFETIME,
            _ => false,
        }
    }
}

/// Rocket WebSocket handler — принимает pub_vtns как идентификатор сессии в пути.
/// Клиент обязан предварительно сдать зашифрованную пару ВТНС через HTTP до открытия сокета.
/// После авторизации (отправки валидных данных в сокете) сессия переходит в Authenticated.
/// Route: GET /api/hnts/ws/<pub_vtns>
#[get("/ws/<pub_vtns>")]
pub fn ws(ws: WebSocket, pub_vtns: String, hnts: &State<HntsState>) -> Channel<'static> {
    let hnts = hnts.inner().clone();
    ws.channel(move |stream| Box::pin(async move {
        let (mut sink, mut source) = stream.split();
        let (tx, mut rx) = mpsc::channel::<Message>(32);

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // TODO: найти существующую сессию по pub_vtns и убедиться что она в состоянии LongToken.
        //       Пока создаём сессию здесь — после реализации хранилища сессий заменить на lookup.
        let _ = (pub_vtns, hnts.is_valid("").await);
        let mut session = Session::new(tx);

        while let Some(result) = source.next().await {
            let msg = match result {
                Ok(m) => m,
                                      Err(_) => break,
            };

            if session.tokens_expired() {
                // TODO: в состоянии LongToken — сгенерировать новую пару ВТНС;
                //       в состоянии Authenticated — сгенерировать новую пару ВПубАТ+ВПривАТ,
                //       зашифровать старым ВПривАТ и отправить через session.send_binary()
            }

            session.on_message(msg, |sess, msg| Box::pin(async move {
                match &sess.state {
                    SessionState::Entrypoint => {
                        // TODO: декриптовать JWE(серверный ВТНС, {pub_vtns, priv_vtns}),
                        //       валидировать серверный ВТНС через HntsState::is_valid(),
                        //       перевести сессию в LongToken { public_vtns, private_vtns, issued_at }
                        let _ = msg;
                    }
                    SessionState::LongToken { .. } => {
                        // TODO: декриптовать сообщение с помощью private_vtns;
                        //       если данные аутентификации валидны — сгенерировать ВПубАТ+ВПривАТ
                        //       и перевести сессию в Authenticated { pub_at, priv_at, issued_at }
                    }
                    SessionState::Authenticated { .. } => {
                        // TODO: декриптовать сообщение с помощью priv_at;
                        //       при ошибке дешифровки перевести сессию в PrivateOnly(priv_at)
                        let _ = sess.send("ok").await;
                    }
                    SessionState::PrivateOnly(_) => {
                        // TODO: принять хеш ВПривАТ, выдать новый ВПубАТ → Authenticated;
                        //       при неудаче — сбросить сессию в Entrypoint
                    }
                }
            })).await;
        }

        Ok(())
    }))
}
