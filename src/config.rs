use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub irc: IrcConfig,
    pub discord: DiscordConfig,
}

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct IrcConfig {
    pub host: String,
    pub port: u16,
    pub real_name: String,
    pub nick: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
    }
}
