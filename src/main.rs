use crate::{cli::Cli, irc::IrcClient};
use anyhow::Result;
use clap::Parser;

mod cli;
mod irc;

// TODO: Make these configurable via the CLI.
const ADDR: &str = "irc.rizon.net";
const PORT: u16 = 6667;

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = Cli::parse();
    let mut client = IrcClient::builder()
        .host(ADDR)
        .port(PORT)
        .nick_name("percy")
        .real_name("percy")
        .build();

    // Connect to IRC.
    client.connect().await?;

    // Wait until the user presses Ctrl+C to exit.
    tokio::signal::ctrl_c().await?;

    Ok(())
}
