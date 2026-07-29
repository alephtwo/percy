use crate::{cli::Cli, config::Config, irc::IrcClient};
use anyhow::Result;
use clap::Parser;

mod cli;
mod config;
mod irc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;

    let mut irc = IrcClient::builder()
        .host(&config.irc.host)
        .port(config.irc.port)
        .nick_name(&config.irc.nick)
        .real_name(&config.irc.real_name)
        .build();

    // Connect to IRC.
    irc.connect().await?;

    // Wait until the user presses Ctrl+C to exit.
    tokio::signal::ctrl_c().await?;

    Ok(())
}
