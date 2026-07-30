use crate::{
    bridge::{BridgeMessage, MessageSource},
    config::DiscordConfig,
};
use anyhow::Result;
use serenity::{
    Client,
    all::{
        Context, CreateBotAuthParameters, EventHandler, GatewayIntents, Message, Permissions,
        Ready, Scope,
    },
    async_trait,
};
use tokio::sync::mpsc;

/// Creates a Discord client using the provided configuration and IRC bridge channel.
pub async fn get_client(
    config: &DiscordConfig,
    discord_token: &str,
    irc_tx: mpsc::Sender<BridgeMessage>,
) -> Result<Client> {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(discord_token, intents)
        .event_handler(DiscordHandler {
            irc_tx,
            channel_id: config.channel_id,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

/// Handles incoming Discord messages and sends them to the IRC bridge.
struct DiscordHandler {
    channel_id: u64,
    // Store
    irc_tx: mpsc::Sender<BridgeMessage>,
}

#[async_trait]
impl EventHandler for DiscordHandler {
    /// Handles the ready event, generating an invite link and printing it out.
    async fn ready(&self, ctx: Context, _: Ready) {
        // Generate an invite link and print it out
        let builder = match CreateBotAuthParameters::new()
            .permissions(Permissions::SEND_MESSAGES)
            .scopes(&[Scope::Bot])
            .auto_client_id(&ctx.http)
            .await
        {
            Ok(builder) => builder,
            Err(builder) => {
                eprintln!("Failed to generate invite link: {}", builder);
                return;
            }
        };

        println!("Invite Link: {}", builder.build());
    }

    /// Handles incoming messages and sends them to the IRC bridge.
    async fn message(&self, ctx: Context, msg: Message) {
        // Filter out messages sent by our Discord bot
        if ctx.cache.current_user().id == msg.author.id.get() {
            return;
        }

        // Filter out: wrong channel
        if msg.channel_id.get() != self.channel_id {
            return;
        }

        // Send the message to the IRC bridge
        let bridge_msg = BridgeMessage {
            author: msg.author.name,
            content: msg.content,
            source: MessageSource::Discord,
        };
        match self.irc_tx.send(bridge_msg).await {
            Ok(_) => {}
            Err(e) => eprintln!("Failed to send message to IRC: {}", e),
        }
    }
}
