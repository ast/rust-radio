use std::io::{self, Write};
use std::path::Path;

use anyhow::{Result, bail};

use crate::auth;
use crate::config::{Config, UserRecord};

pub fn run(config_path: &Path, username: &str) -> Result<()> {
    let mut config = Config::load(config_path)?;

    if config.users.iter().any(|u| u.username == username) {
        bail!("user '{username}' already exists");
    }

    let password = read_password("Password: ")?;
    let password_hash = auth::hash_password(&password)?;

    config.users.push(UserRecord {
        username: username.to_string(),
        password_hash,
    });

    config.save(config_path)?;
    println!("added user '{username}'");
    Ok(())
}

fn read_password(prompt: &str) -> Result<String> {
    eprint!("{prompt}");
    io::stderr().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    Ok(password.trim().to_string())
}
