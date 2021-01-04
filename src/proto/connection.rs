use std::io;
use std::marker::PhantomData;

use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::proto::{FrameDecoder, FrameEncoder, ValueDecode, ValueEncode};

#[derive(Debug)]
pub struct Connection<ReadFrame, WriteFrame: ?Sized> {
    stream: TcpStream,

    read_buffer: BytesMut,

    phantom_read: PhantomData<ReadFrame>,
    phantom_write: PhantomData<WriteFrame>,
}

impl<ReadFrame, WriteFrame> Connection<ReadFrame, WriteFrame>
where
    ReadFrame: ValueDecode,
    WriteFrame: ValueEncode + ?Sized,
{
    pub fn new(stream: TcpStream) -> Self {
        Connection {
            stream,
            read_buffer: BytesMut::with_capacity(4096),
            phantom_read: PhantomData,
            phantom_write: PhantomData,
        }
    }

    pub async fn read(&mut self) -> io::Result<ReadFrame> {
        let mut decoder = FrameDecoder::new();

        loop {
            if let Some(frame) = decoder.decode_from(&mut self.read_buffer)? {
                return Ok(frame);
            }
            self.stream.read_buf(&mut self.read_buffer).await?;
        }
    }

    pub async fn write(&mut self, frame: &WriteFrame) -> io::Result<()> {
        let mut bytes = BytesMut::new();
        FrameEncoder::new().encode_to(frame, &mut bytes)?;
        self.stream.write_all(bytes.as_ref()).await
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::{TcpListener, TcpStream};

    use super::Connection;

    #[tokio::test]
    async fn ping_pong() {
        let listener = TcpListener::bind("localhost:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (stream, _peer_address) = listener.accept().await.unwrap();
            let mut connection = Connection::<String, str>::new(stream);

            assert_eq!(connection.read().await.unwrap(), "ping");
            connection.write("pong").await.unwrap();
            assert_eq!(connection.read().await.unwrap(), "ping");
            connection.write("pong").await.unwrap();
        });

        let stream = TcpStream::connect(address).await.unwrap();
        let mut connection = Connection::<String, str>::new(stream);

        connection.write("ping").await.unwrap();
        assert_eq!(connection.read().await.unwrap(), "pong");
        connection.write("ping").await.unwrap();
        assert_eq!(connection.read().await.unwrap(), "pong");

        server_task.await.unwrap();
    }
}
