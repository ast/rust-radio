use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

pub struct Session {
    pub username: String,
    pub created_at: Instant,
}

pub struct SessionStore {
    sessions: RwLock<HashMap<String, Session>>,
    ttl: Duration,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore {
    pub fn new() -> Self {
        Self::with_ttl(SESSION_TTL)
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    pub async fn insert(&self, token: String, username: String) {
        let session = Session {
            username,
            created_at: Instant::now(),
        };
        self.sessions.write().await.insert(token, session);
    }

    pub async fn username(&self, token: &str) -> Option<String> {
        let mut sessions = self.sessions.write().await;
        let expired = sessions
            .get(token)
            .is_some_and(|s| s.created_at.elapsed() >= self.ttl);
        if expired {
            sessions.remove(token);
            return None;
        }
        sessions.get(token).map(|s| s.username.clone())
    }

    pub async fn remove(&self, token: &str) {
        self.sessions.write().await.remove(token);
    }

    pub async fn remove_expired(&self) {
        let ttl = self.ttl;
        self.sessions
            .write()
            .await
            .retain(|_, s| s.created_at.elapsed() < ttl);
    }
}
