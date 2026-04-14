use std::path::Path;

use anyhow::Result;

use crate::config::Config;

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
