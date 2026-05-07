use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::secure;

pub struct InviteTokenStore {
    inner: RwLock<HashSet<String>>,
}

impl InviteTokenStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { inner: RwLock::new(HashSet::new()) })
    }

    pub async fn create_token(&self) -> String {
        let token = secure::get_token(24);
        self.inner.write().await.insert(token.clone());
        token
    }

    pub async fn consume_token(&self, token: &str) -> bool {
        self.inner.write().await.remove(token)
    }

    pub async fn list_tokens(&self) -> Vec<String> {
        self.inner.read().await.iter().cloned().collect()
    }
}
