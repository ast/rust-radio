// Copyright SM6WJM 2026

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub radio: RadioConfig,
    #[serde(default)]
    pub users: Vec<UserRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioConfig {
    pub url: String,
    pub audio_device: Option<String>,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
}

fn default_sample_rate() -> u32 {
    48000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub username: String,
    pub password_hash: String,
}

impl Config {
    pub fn load(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::CivlinkError::Config(format!("failed to read config: {e}")))?;
        toml::from_str(&content)
            .map_err(|e| crate::CivlinkError::Config(format!("failed to parse config: {e}")))
    }

    pub fn save(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::CivlinkError::Config(format!("failed to serialize config: {e}")))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
