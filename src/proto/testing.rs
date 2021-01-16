//! Provides utilities for testing protocol code.

use std::io;
use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream};

use crate::proto::{Connection, ServerRequest, ServerResponse};

async fn process(stream: TcpStream) -> io::Result<()> {
    let mut connection =
        Connection::<ServerRequest, ServerResponse>::new(stream);

    let _request = match connection.read().await? {
        ServerRequest::LoginRequest(request) => request,
        request => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("expected login request, got: {:?}", request),
            ));
        }
    };

    Ok(())
}

/// A fake server for connecting to in tests.
pub struct FakeServer {
    listener: TcpListener,
}

impl FakeServer {
    /// Creates a new fake server and binds it to a port on localhost.
    pub async fn new() -> io::Result<Self> {
        let listener = TcpListener::bind("localhost:0").await?;
        Ok(FakeServer { listener })
    }

    /// Returns the address to which this server is bound.
    /// This is always localhost and a random port chosen by the OS.
    pub fn address(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Runs the server: accepts incoming connections and responds to requests.
    pub async fn run(&mut self) -> io::Result<()> {
        loop {
            let (socket, _peer_address) = self.listener.accept().await?;
            tokio::spawn(async move { process(socket).await });
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpStream;

    use super::FakeServer;

    #[tokio::test]
    async fn new_binds_to_localhost() {
        let server = FakeServer::new().await.unwrap();
        assert!(server.address().unwrap().ip().is_loopback());
    }

    #[tokio::test]
    async fn accepts_incoming_connections() {
        let mut server = FakeServer::new().await.unwrap();
        let address = server.address().unwrap();
        tokio::spawn(async move { server.run().await.unwrap() });

        // The connection succeeds.
        let _ = TcpStream::connect(address).await.unwrap();
    }
}
