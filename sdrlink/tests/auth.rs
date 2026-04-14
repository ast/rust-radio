use sdrlink::auth;

#[test]
fn hash_and_verify_password() {
    let hash = auth::hash_password("correcthorse").unwrap();
    assert!(hash.starts_with("$argon2id$"));
    assert!(auth::verify_password("correcthorse", &hash).unwrap());
}

#[test]
fn wrong_password_does_not_verify() {
    let hash = auth::hash_password("correcthorse").unwrap();
    assert!(!auth::verify_password("wrongpassword", &hash).unwrap());
}

#[test]
fn different_passwords_produce_different_hashes() {
    let h1 = auth::hash_password("password1").unwrap();
    let h2 = auth::hash_password("password2").unwrap();
    assert_ne!(h1, h2);
}

#[test]
fn same_password_produces_different_hashes_due_to_salt() {
    let h1 = auth::hash_password("samepassword").unwrap();
    let h2 = auth::hash_password("samepassword").unwrap();
    assert_ne!(h1, h2);
    assert!(auth::verify_password("samepassword", &h1).unwrap());
    assert!(auth::verify_password("samepassword", &h2).unwrap());
}

#[test]
fn generate_token_is_unique() {
    let t1 = auth::generate_token();
    let t2 = auth::generate_token();
    assert_ne!(t1, t2);
    assert!(!t1.is_empty());
}

#[test]
fn invalid_hash_returns_error() {
    let result = auth::verify_password("password", "not-a-valid-hash");
    assert!(result.is_err());
}
