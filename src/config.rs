use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub irc: IrcConfig,
    pub discord: DiscordConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiscordConfig {
    pub token: String,
    pub channel_id: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IrcConfig {
    pub host: String,
    pub port: u16,
    pub channel: IrcChannelConfig,

    pub real_name: String,
    pub nick: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IrcChannelConfig {
    pub name: String,
    pub password: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
    }
}
