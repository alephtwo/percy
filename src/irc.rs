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
    connected: bool,
}

impl IrcClient {
    /// Creates a builder to configure and construct an [`IrcClient`].
    pub fn builder() -> IrcClientBuilder {
        IrcClientBuilder::default()
    }

    /// Connects to the IRC server and spawns the background processing task.
    ///
    /// Returns a tuple containing:
    /// - An active command handle (`IrcClientHandle`) to send commands (JOIN, PRIVMSG, etc.).
    /// - An `mpsc::Receiver<IrcEvent>` to listen to incoming events from the server.
    pub async fn connect(&mut self) -> Result<(IrcClientHandle, mpsc::Receiver<IrcEvent>)> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(&addr).await?;
        let (reader, mut writer) = stream.into_split();

        // Channel for outgoing command lines to the TCP writer task
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(100);
        // Channel for incoming IRC events emitted to the caller
        let (event_tx, event_rx) = mpsc::channel::<IrcEvent>(100);

        // Perform initial registration handshake
        writer
            .write_all(format!("NICK {}\r\n", self.nick).as_bytes())
            .await?;
        writer
            .write_all(format!("USER {} 0 * :{}\r\n", self.nick, self.real_name).as_bytes())
            .await?;

        self.connected = true;
        let nick = self.nick.clone();

        // Spawn background reader/writer loop
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();

            loop {
                tokio::select! {
                    // --- 1. INCOMING FROM IRC SERVER ---
                    line_res = lines.next_line() => {
                        let line = match line_res {
                            Ok(Some(l)) => l,
                            Ok(None) => break, // Server closed connection
                            Err(e) => {
                                let _ = event_tx.send(IrcEvent::Error(e.to_string())).await;
                                break;
                            }
                        };

                        // Auto-handle PINGs internally to keep connection alive
                        if line.starts_with("PING") {
                            let payload = line.trim_start_matches("PING ").to_string();
                            let _ = writer.write_all(format!("PONG {}\r\n", payload).as_bytes()).await;
                            // No need to emit Ping event, handled internally
                            continue;
                        }

                        // Emit Connected event when RPL_WELCOME (001) is received
                        if line.contains(" 001 ") {
                            let _ = event_tx.send(IrcEvent::Connected).await;
                        }

                        // Parse PRIVMSGs: :nick!user@host PRIVMSG #channel :message
                        if line.contains(" PRIVMSG ") {
                          if let Some((sender, target, text)) = parse_privmsg(&line) {
                              if sender != nick {
                                  let _ = event_tx
                                      .send(IrcEvent::Message {
                                          sender: sender.to_string(),
                                          target: target.to_string(),
                                          content: text.to_string(),
                                      })
                                      .await;
                              }
                          }
                        }
                    }

                    // --- 2. OUTGOING COMMANDS TO IRC SERVER ---
                    Some(cmd) = cmd_rx.recv() => {
                        let mut bytes = cmd.into_bytes();
                        // IRC protocol requires lines terminated by CRLF (\r\n)
                        bytes.extend_from_slice(b"\r\n");
                        if let Err(e) = writer.write_all(&bytes).await {
                            let _ = event_tx.send(IrcEvent::Error(e.to_string())).await;
                            break;
                        }
                    }
                }
            }

            let _ = event_tx.send(IrcEvent::Closed).await;
        });

        let handle = IrcClientHandle { cmd_tx };
        Ok((handle, event_rx))
    }
}

/// A lightweight handle to send commands to the active IRC connection.
#[derive(Clone)]
pub struct IrcClientHandle {
    cmd_tx: mpsc::Sender<String>,
}

impl IrcClientHandle {
    /// Joins a target channel (e.g., `"#rust"`).
    pub async fn join(&self, channel: &str, password: Option<String>) -> Result<()> {
        self.cmd_tx
            .send(match password {
                Some(p) => format!("JOIN {} {}", channel, p),
                None => format!("JOIN {}", channel),
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send JOIN command: {}", e))
    }

    /// Sends a PRIVMSG to a target channel or user.
    pub async fn send_message(&self, target: &str, message: &str) -> Result<()> {
        // Strip newlines to adhere to IRC protocol single-line limit
        let sanitized_msg = message.replace(['\r', '\n'], " ");
        self.cmd_tx
            .send(format!("PRIVMSG {} :{}", target, sanitized_msg))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send PRIVMSG command: {}", e))
    }
}

/// Parses standard IRC PRIVMSG lines.
/// Line format: `:nick!user@host PRIVMSG #channel :hello world`
fn parse_privmsg(line: &str) -> Option<(&str, &str, &str)> {
    if !line.starts_with(':') {
        return None;
    }

    let mut parts = line[1..].splitn(2, " PRIVMSG ");
    let prefix = parts.next()?;
    let rest = parts.next()?;

    let sender = prefix.split('!').next()?;

    let mut rest_parts = rest.splitn(2, " :");
    let target = rest_parts.next()?.trim();
    let text = rest_parts.next()?;

    Some((sender, target, text))
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
    Message {
        sender: String,
        target: String,
        content: String,
    },
    Error(String),
    Closed,
}
