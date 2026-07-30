use crate::{
    bridge::{BridgeMessage, MessageSource},
    cli::Cli,
    config::{Config, DiscordConfig, IrcConfig},
    irc::{IrcClient, IrcEvent},
};
use anyhow::Result;
use clap::Parser;
use serenity::all::{ChannelId, Http};
use std::sync::Arc;
use tokio::sync::mpsc;

mod bridge;
mod cli;
mod config;
mod discord;
mod irc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;

    // Channel 1: Discord -> IRC
    let (irc_tx, irc_rx) = tokio::sync::mpsc::channel::<BridgeMessage>(100);
    // Channel 2: IRC -> Discord
    let (discord_tx, discord_rx) = tokio::sync::mpsc::channel::<BridgeMessage>(100);

    // Build a Discord client, giving it the IRC tx channel so it can send messages to IRC
    let mut discord = discord::get_client(&config.discord, irc_tx).await?;
    let discord_http = discord.http.clone();

    // Spawn tasks
    tokio::spawn(send_irc_to_discord(
        config.discord.clone(),
        discord_http,
        discord_rx,
    ));
    tokio::spawn(send_discord_to_irc(config.irc.clone(), irc_rx, discord_tx));

    println!("Starting Discord Gateway...");
    discord.start().await?;

    Ok(())
}

/// Listens for messages from the Discord client and sends them to the IRC channel
async fn send_irc_to_discord(
    config: DiscordConfig,
    http: Arc<Http>,
    mut discord_rx: mpsc::Receiver<BridgeMessage>,
) {
    let channel_id = ChannelId::new(config.channel_id);
    while let Some(msg) = discord_rx.recv().await {
        if msg.source != MessageSource::Irc {
            return;
        }

        // TODO: Build out the implementation to put this in the right discord channel
        let output = format!("**<{}@irc>** {}", msg.author, msg.content);

        // Send via Serenity's HTTP client
        if let Err(e) = channel_id.say(&http, &output).await {
            eprintln!(
                "[Discord Error] Failed to send message to channel {}: {:?}",
                config.channel_id, e
            );
        }
    }
}

/// Listens for messages from the IRC client and sends them to the Discord channel
async fn send_discord_to_irc(
    config: IrcConfig,
    mut irc_rx: mpsc::Receiver<BridgeMessage>,
    discord_tx: mpsc::Sender<BridgeMessage>,
) {
    // 1. Build and configure the IRC client using config
    let mut client = IrcClient::builder()
        .host(&config.host)
        .port(config.port)
        .nick_name(&config.nick)
        .real_name(&config.real_name)
        .build();

    // 2. Connect to the IRC server
    let (irc_handle, mut irc_events) = match client.connect().await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to connect to IRC server: {:?}", e);
            return;
        }
    };

    println!("Connected to IRC server at {}:{}", config.host, config.port);

    // 3. Event Select Loop
    loop {
        tokio::select! {
            // --- INCOMING EVENTS FROM IRC ---
            Some(event) = irc_events.recv() => {
                match event {
                    IrcEvent::Connected => {
                        println!("IRC Registration complete. Joining channel {}", config.channel.name);
                        if let Err(e) = irc_handle.join(&config.channel.name, config.channel.password.clone()).await {
                            eprintln!("Failed to send JOIN command: {:?}", e);
                        }
                    }
                    IrcEvent::Message { sender, target, content } => {
                        // Forward to Discord only if message came from our target IRC channel
                        if target == config.channel.name {
                            let bridge_msg = BridgeMessage {
                                author: sender,
                                content,
                                source: MessageSource::Irc,
                            };

                            let _ = discord_tx.send(bridge_msg).await;
                        }
                    }
                    IrcEvent::Error(err) => {
                        eprintln!("[IRC Error] {}", err);
                    }
                    IrcEvent::Closed => {
                        println!("IRC Connection closed.");
                        break;
                    }
                }
            }

            // --- INCOMING MESSAGES FROM DISCORD TO RELAY TO IRC ---
            Some(msg) = irc_rx.recv() => {
                if msg.source == MessageSource::Discord {
                    let text = format!("<{}> {}", msg.author, msg.content);
                    if let Err(e) = irc_handle.send_message(&config.channel.name, &text).await {
                        eprintln!("Failed to send message to IRC channel: {:?}", e);
                    }
                }
            }
        }
    }
}
