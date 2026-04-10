// Copyright SM6WJM 2026

use std::path::Path;

use anyhow::{bail, Result};

use crate::config::Config;

pub fn run(config_path: &Path, username: &str) -> Result<()> {
    let mut config = Config::load(config_path)?;

    let before = config.users.len();
    config.users.retain(|u| u.username != username);

    if config.users.len() == before {
        bail!("user '{username}' not found");
    }

    config.save(config_path)?;
    println!("removed user '{username}'");
    Ok(())
}
