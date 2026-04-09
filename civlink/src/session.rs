// Copyright SM6WJM 2026

use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;

pub struct Session {
    pub username: String,
    pub created_at: Instant,
}

pub struct SessionStore {
    sessions: RwLock<HashMap<String, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn insert(&self, token: String, username: String) {
        let session = Session {
            username,
            created_at: Instant::now(),
        };
        self.sessions.write().await.insert(token, session);
    }

    pub async fn get_username(&self, token: &str) -> Option<String> {
        self.sessions
            .read()
            .await
            .get(token)
            .map(|s| s.username.clone())
    }

    pub async fn remove(&self, token: &str) {
        self.sessions.write().await.remove(token);
    }
}
