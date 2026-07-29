use crate::{
    cli::Cli,
    config::Config,
    irc::{IrcClient, IrcEvent},
};
use anyhow::Result;
use clap::Parser;

mod cli;
mod config;
mod irc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;

    let irc = IrcClient::builder()
        .host(&config.irc.host)
        .port(config.irc.port)
        .nick_name(&config.irc.nick)
        .real_name(&config.irc.real_name)
        .build();

    // Connect to IRC.
    let mut irc_events = irc.connect().await?;
    while let Some(event) = irc_events.recv().await {
        match event {
            IrcEvent::Connected => println!("🚀 Connected!"),
            IrcEvent::Ping(payload) => {
                println!("🏓 Heartbeat PING received: {payload} (PONG auto-sent)");
            }
            IrcEvent::Message(msg) => println!("📩 Received: {msg}"),
            IrcEvent::Error(err) => eprintln!("⚠️ Error: {err}"),
            IrcEvent::Closed => {
                println!("🔌 Disconnected.");
                break;
            }
        }
    }

    // Wait until the user presses Ctrl+C to exit.
    tokio::signal::ctrl_c().await?;

    Ok(())
}
