use crate::bridge::{BridgeMessage, MessageSource};
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

pub async fn get_client(token: &String, irc_tx: mpsc::Sender<BridgeMessage>) -> Result<Client> {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    Client::builder(token, intents)
        .event_handler(DiscordHandler { irc_tx })
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

struct DiscordHandler {
    irc_tx: mpsc::Sender<BridgeMessage>,
}

#[async_trait]
impl EventHandler for DiscordHandler {
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

    async fn message(&self, ctx: Context, msg: Message) {
        // Filter out: wrong channel OR messages sent by our Discord bot
        if msg.channel_id.get() != self.config.target_discord_channel_id {
            return;
        }
        if msg.author.id.get() == self.config.discord_bot_user_id {
            return;
        }

        let bridge_msg = BridgeMessage {
            author: msg.author.name,
            content: msg.content,
            source: MessageSource::Discord,
        };
        self.irc_tx.send(bridge_msg).await;
    }
}
