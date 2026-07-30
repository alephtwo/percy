#[derive(Debug, Clone)]
pub struct BridgeMessage {
    pub author: String,
    pub content: String,
    pub source: MessageSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageSource {
    Discord,
    Irc,
}
