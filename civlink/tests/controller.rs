// Copyright SM6WJM 2026

use civlink::radio::Controller;

#[tokio::test]
async fn acquire_and_release_control() {
    let ctrl = Controller::new();

    assert!(ctrl.try_acquire("sm6wjm").await);
    assert_eq!(ctrl.current().await, Some("sm6wjm".to_string()));

    ctrl.release("sm6wjm").await;
    assert_eq!(ctrl.current().await, None);
}

#[tokio::test]
async fn only_one_controller_at_a_time() {
    let ctrl = Controller::new();

    assert!(ctrl.try_acquire("user1").await);
    assert!(!ctrl.try_acquire("user2").await);

    assert_eq!(ctrl.current().await, Some("user1".to_string()));
}

#[tokio::test]
async fn wrong_user_cannot_release() {
    let ctrl = Controller::new();

    ctrl.try_acquire("user1").await;
    ctrl.release("user2").await; // should be a no-op

    assert_eq!(ctrl.current().await, Some("user1".to_string()));
}

#[tokio::test]
async fn reacquire_after_release() {
    let ctrl = Controller::new();

    ctrl.try_acquire("user1").await;
    ctrl.release("user1").await;

    assert!(ctrl.try_acquire("user2").await);
    assert_eq!(ctrl.current().await, Some("user2".to_string()));
}

#[tokio::test]
async fn no_controller_initially() {
    let ctrl = Controller::new();
    assert_eq!(ctrl.current().await, None);
}
