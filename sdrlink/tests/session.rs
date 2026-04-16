use std::time::Duration;

use sdrlink::session::SessionStore;

#[tokio::test]
async fn insert_and_retrieve_session() {
    let store = SessionStore::new();
    store
        .insert("token123".to_string(), "sm6wjm".to_string())
        .await;

    assert_eq!(store.username("token123").await, Some("sm6wjm".to_string()));
}

#[tokio::test]
async fn unknown_token_returns_none() {
    let store = SessionStore::new();
    assert_eq!(store.username("nonexistent").await, None);
}

#[tokio::test]
async fn remove_session() {
    let store = SessionStore::new();
    store
        .insert("token123".to_string(), "sm6wjm".to_string())
        .await;
    store.remove("token123").await;

    assert_eq!(store.username("token123").await, None);
}

#[tokio::test]
async fn expired_token_returns_none() {
    let store = SessionStore::with_ttl(Duration::from_millis(20));
    store
        .insert("token123".to_string(), "sm6wjm".to_string())
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(store.username("token123").await, None);
    // A fresh insert under the same token should be visible again.
    store
        .insert("token123".to_string(), "sm6wjm".to_string())
        .await;
    assert_eq!(store.username("token123").await, Some("sm6wjm".to_string()));
}

#[tokio::test]
async fn remove_expired_drops_old_sessions() {
    let store = SessionStore::with_ttl(Duration::from_millis(20));
    store.insert("t1".to_string(), "user1".to_string()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    store.insert("t2".to_string(), "user2".to_string()).await;
    store.remove_expired().await;
    assert_eq!(store.username("t1").await, None);
    assert_eq!(store.username("t2").await, Some("user2".to_string()));
}

#[tokio::test]
async fn multiple_sessions() {
    let store = SessionStore::new();
    store.insert("t1".to_string(), "user1".to_string()).await;
    store.insert("t2".to_string(), "user2".to_string()).await;

    assert_eq!(store.username("t1").await, Some("user1".to_string()));
    assert_eq!(store.username("t2").await, Some("user2".to_string()));
}
