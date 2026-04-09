// Copyright SM6WJM 2026

use std::path::Path;

use crate::config::Config;
use crate::Result;

pub fn run(config_path: &Path) -> Result<()> {
    let config = Config::load(config_path)?;

    if config.users.is_empty() {
        println!("no users configured");
    } else {
        for user in &config.users {
            println!("{}", user.username);
        }
    }

    Ok(())
}
