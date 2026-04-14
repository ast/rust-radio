use sdrlink::config::{Config, SdrConfig, ServerConfig, UserRecord};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn load_and_save_round_trip() {
    let config = Config {
        server: ServerConfig {
            listen_addr: "0.0.0.0:8081".to_string(),
        },
        sdr: SdrConfig {
            center_hz: 100_300_000.0,
            samplerate: 768_000,
            fft_len: 32_768,
            fft_rate_hz: 20,
        },
        users: vec![UserRecord {
            username: "sm6wjm".to_string(),
            password_hash: "$argon2id$fake".to_string(),
        }],
    };

    let tmp = NamedTempFile::new().unwrap();
    config.save(tmp.path()).unwrap();

    let loaded = Config::load(tmp.path()).unwrap();
    assert_eq!(loaded.server.listen_addr, "0.0.0.0:8081");
    assert_eq!(loaded.sdr.center_hz, 100_300_000.0);
    assert_eq!(loaded.sdr.samplerate, 768_000);
    assert_eq!(loaded.sdr.fft_len, 32_768);
    assert_eq!(loaded.users.len(), 1);
    assert_eq!(loaded.users[0].username, "sm6wjm");
}

#[test]
fn load_minimal_config_uses_defaults() {
    let toml = r#"
[server]
listen_addr = "127.0.0.1:3000"

[sdr]
center_hz = 7_200_000.0
"#;
    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(toml.as_bytes()).unwrap();

    let config = Config::load(tmp.path()).unwrap();
    assert_eq!(config.server.listen_addr, "127.0.0.1:3000");
    assert_eq!(config.sdr.center_hz, 7_200_000.0);
    assert_eq!(config.sdr.samplerate, 768_000);
    assert_eq!(config.sdr.fft_len, 32_768);
    assert_eq!(config.sdr.fft_rate_hz, 20);
    assert!(config.users.is_empty());
}

#[test]
fn load_missing_file_returns_error() {
    let result = Config::load(std::path::Path::new("/tmp/nonexistent_sdrlink.toml"));
    assert!(result.is_err());
}

#[test]
fn load_invalid_toml_returns_error() {
    let mut tmp = NamedTempFile::new().unwrap();
    tmp.write_all(b"this is not valid toml {{{}}}").unwrap();

    let result = Config::load(tmp.path());
    assert!(result.is_err());
}
