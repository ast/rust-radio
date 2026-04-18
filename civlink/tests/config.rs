// Copyright SM6WJM 2026

use civlink::config::{Config, RadioConfig, ServerConfig, UserRecord};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn load_and_save_round_trip() {
    let config = Config {
        server: ServerConfig {
            listen_addr: "0.0.0.0:8080".to_string(),
        },
        radio: RadioConfig {
            url: "tcp://shack:9000".to_string(),
            audio_device: Some("USB Audio CODEC".to_string()),
        },
        users: vec![UserRecord {
            username: "sm6wjm".to_string(),
            password_hash: "$argon2id$fake".to_string(),
        }],
    };

    let tmp = NamedTempFile::new().unwrap();
    config.save(tmp.path()).unwrap();

    let loaded = Config::load(tmp.path()).unwrap();
    assert_eq!(loaded.server.listen_addr, "0.0.0.0:8080");
    assert_eq!(loaded.radio.url, "tcp://shack:9000");
    assert_eq!(loaded.radio.audio_device.as_deref(), Some("USB Audio CODEC"));
    assert_eq!(loaded.users.len(), 1);
    assert_eq!(loaded.users[0].username, "sm6wjm");
}

#[test]
fn load_minimal_config() {
    let toml = r#"
[server]
listen_addr = "127.0.0.1:3000"

[radio]
url = "serial:///dev/ttyUSB0"
"#;
    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(toml.as_bytes()).unwrap();

    let config = Config::load(tmp.path()).unwrap();
    assert_eq!(config.server.listen_addr, "127.0.0.1:3000");
    assert_eq!(config.radio.audio_device, None);
    assert!(config.users.is_empty());
}

#[test]
fn load_missing_file_returns_error() {
    let result = Config::load(std::path::Path::new("/tmp/nonexistent_civlink.toml"));
    assert!(result.is_err());
}

#[test]
fn load_invalid_toml_returns_error() {
    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(b"this is not valid toml {{{}}}").unwrap();

    let result = Config::load(tmp.path());
    assert!(result.is_err());
}
