use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::error::SdrLinkError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub sdr: SdrConfig,
    #[serde(default)]
    pub users: Vec<UserRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdrConfig {
    pub center_hz: f64,
    #[serde(default = "default_samplerate")]
    pub samplerate: u32,
    #[serde(default = "default_fft_len")]
    pub fft_len: u32,
    #[serde(default = "default_fft_rate_hz")]
    pub fft_rate_hz: u32,
}

fn default_samplerate() -> u32 {
    768_000
}

fn default_fft_len() -> u32 {
    32_768
}

fn default_fft_rate_hz() -> u32 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub username: String,
    pub password_hash: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SdrLinkError::Config(format!("failed to read config: {e}")))?;
        toml::from_str(&content)
            .map_err(|e| SdrLinkError::Config(format!("failed to parse config: {e}")))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| SdrLinkError::Config(format!("failed to serialize config: {e}")))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
