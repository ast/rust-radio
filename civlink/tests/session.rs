// Copyright SM6WJM 2026

use civlink::session::SessionStore;

#[tokio::test]
async fn insert_and_retrieve_session() {
    let store = SessionStore::new();
    store.insert("token123".to_string(), "sm6wjm".to_string()).await;

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
    store.insert("token123".to_string(), "sm6wjm".to_string()).await;
    store.remove("token123").await;

    assert_eq!(store.username("token123").await, None);
}

#[tokio::test]
async fn multiple_sessions() {
    let store = SessionStore::new();
    store.insert("t1".to_string(), "user1".to_string()).await;
    store.insert("t2".to_string(), "user2".to_string()).await;

    assert_eq!(store.username("t1").await, Some("user1".to_string()));
    assert_eq!(store.username("t2").await, Some("user2".to_string()));
}

#[tokio::test]
async fn remove_expired_keeps_fresh_sessions() {
    let store = SessionStore::new();
    store.insert("token123".to_string(), "sm6wjm".to_string()).await;
    store.remove_expired().await;

    assert_eq!(store.username("token123").await, Some("sm6wjm".to_string()));
}
