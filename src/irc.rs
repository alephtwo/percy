use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

/// Represents an IRC client for connecting to an IRC server.
pub struct IrcClient {
    host: String,
    port: u16,
    nick: String,
    real_name: String,
    connected: bool,
}

impl IrcClient {
    /// Gets a builder for creating an [`IrcClient`].
    pub fn builder() -> IrcClientBuilder {
        IrcClientBuilder::default()
    }

    pub async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(&addr).await?;
        println!("Connected to {}", &addr);

        // Spawn background task to process incoming data
        let (mut reader, mut writer) = stream.into_split();

        // Pass initial payload
        let message = format!(
            "NICK {}\r\nUSER guest 0 * :{}\r\n",
            self.nick, self.real_name
        );
        writer.write_all(message.as_bytes()).await?;

        // Process incoming data
        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];

            loop {
                match reader.read(&mut buffer).await {
                    Ok(n) if n > 0 => {
                        let text = String::from_utf8_lossy(&buffer[..n]);
                        println!("[DEBUG] Incoming: {}", text.trim_end());
                        // Split incoming stream by lines to parse individual commands
                        for line in text.lines() {
                            // Check if the server sent a PING request
                            if line.starts_with("PING") {
                                // Extract the ping payload (e.g., ":2148117722")
                                if let Some(payload) = line.strip_prefix("PING ") {
                                    let pong_msg = format!("PONG {}\r\n", payload);
                                    print!("[DEBUG] Outgoing: {}", pong_msg);

                                    // Send auto-PONG response back to server
                                    if let Err(e) = writer.write_all(pong_msg.as_bytes()).await {
                                        eprintln!("[ERROR] Failed to send PONG: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Ok(_) => {
                        println!("[DEBUG] Disconnected from server.");
                        break;
                    }
                    Err(err) => {
                        eprintln!("[ERROR] Socket error: {}", err);
                        break;
                    }
                }
            }
        });

        Ok(())
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
