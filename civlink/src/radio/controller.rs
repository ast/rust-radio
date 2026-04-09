// Copyright SM6WJM 2026

use tokio::sync::Mutex;

/// Manages exclusive control access — only one client may control the radio at a time.
pub struct Controller {
    current: Mutex<Option<String>>,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    pub async fn try_acquire(&self, username: &str) -> bool {
        let mut current = self.current.lock().await;
        if current.is_none() {
            *current = Some(username.to_string());
            true
        } else {
            false
        }
    }

    pub async fn release(&self, username: &str) {
        let mut current = self.current.lock().await;
        if current.as_deref() == Some(username) {
            *current = None;
        }
    }

    pub async fn current(&self) -> Option<String> {
        self.current.lock().await.clone()
    }
}
