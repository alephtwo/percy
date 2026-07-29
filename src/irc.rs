use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// Represents an IRC client for connecting to an IRC server.
pub struct IrcClient {
    host: String,
    port: u16,
    nick: String,
    real_name: String,
    #[allow(dead_code)]
    connected: bool,
}

impl IrcClient {
    /// Gets a builder for creating an [`IrcClient`].
    pub fn builder() -> IrcClientBuilder {
        IrcClientBuilder::default()
    }

    /// Connects to the IRC server and returns an async Receiver channel end.
    /// The background task emits events (Connected, Message, Ping, Error, Closed)
    /// through this channel.
    pub async fn connect(&self) -> Result<mpsc::Receiver<IrcEvent>> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(&addr).await?;

        // 1. Create channel with capacity for 100 buffered events
        let (tx, rx) = mpsc::channel::<IrcEvent>(100);

        // 2. Split TCP stream into reader and writer halves
        let (reader, mut writer) = stream.into_split();

        // 3. Send initial IRC registration payload
        let init_msg = format!(
            "NICK {}\r\nUSER guest 0 * :{}\r\n",
            self.nick, self.real_name
        );
        writer.write_all(init_msg.as_bytes()).await?;

        // 4. Emit the initial 'Connected' event
        let _ = tx.send(IrcEvent::Connected).await;

        // 5. Spawn background reader task to process incoming TCP data
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            loop {
                match lines.next_line().await {
                    // EVENT: Line received from server
                    Ok(Some(line)) => {
                        // Check for PING request
                        if line.starts_with("PING") {
                            if let Some(payload) = line.strip_prefix("PING ") {
                                let payload_str = payload.to_string();

                                // Emit Ping event to user event loop
                                if tx.send(IrcEvent::Ping(payload_str.clone())).await.is_err() {
                                    // Receiver was dropped by the main application thread
                                    break;
                                }

                                // Send auto-PONG reply back to IRC server
                                let pong_msg = format!("PONG {}\r\n", payload_str);
                                if let Err(e) = writer.write_all(pong_msg.as_bytes()).await {
                                    let _ = tx.send(IrcEvent::Error(e.to_string())).await;
                                    break;
                                }
                            }
                            continue;
                        }

                        // Emit standard message event
                        if tx.send(IrcEvent::Message(line)).await.is_err() {
                            break;
                        }
                    }
                    // EVENT: Connection closed gracefully by remote end
                    Ok(None) => {
                        let _ = tx.send(IrcEvent::Closed).await;
                        break;
                    }
                    // EVENT: Socket I/O error
                    Err(err) => {
                        let _ = tx.send(IrcEvent::Error(err.to_string())).await;
                        let _ = tx.send(IrcEvent::Closed).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// Builder for creating an [`IrcClient`].
pub struct IrcClientBuilder {
    host: String,
    port: u16,
    nick: String,
    real_name: String,
}

impl Default for IrcClientBuilder {
    /// Creates a default [`IrcClientBuilder`] with the following settings:
    /// - `host`: `127.0.0.1`
    /// - `port`: `6667`
    /// - `nick`: `guest`
    /// - `real_name`: `guest`
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 6667, // Standard IRC port
            nick: String::from("guest"),
            real_name: String::from("guest"),
        }
    }
}

impl IrcClientBuilder {
    /// Sets the host for the IRC client.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Sets the port for the IRC client.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the nick name for the IRC client.
    pub fn nick_name(mut self, nick: impl Into<String>) -> Self {
        self.nick = nick.into();
        self
    }

    /// Sets the real name for the IRC client.
    pub fn real_name(mut self, real_name: impl Into<String>) -> Self {
        self.real_name = real_name.into();
        self
    }

    /// Consumes the builder and constructs the IrcClient
    pub fn build(self) -> IrcClient {
        IrcClient {
            host: self.host,
            port: self.port,
            nick: self.nick,
            real_name: self.real_name,
            connected: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum IrcEvent {
    Connected,
    Message(String),
    Ping(String),
    Error(String),
    Closed,
}
