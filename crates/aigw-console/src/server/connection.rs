use aigw_core::CryptoCore;
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf};

pub struct Connection {
    writer: OwnedWriteHalf,
    pub(crate) crypto: Option<CryptoCore>,
    pub(crate) cluster: Option<String>,
    pub(crate) ip: Option<String>,
}

impl Connection {
    pub fn new(writer: OwnedWriteHalf) -> Connection {
        Connection {
            writer,
            crypto: None,
            cluster: None,
            ip: None,
        }
    }

    pub async fn write(&mut self, buf: &[u8]) -> anyhow::Result<usize> {
        let size = self.writer.write(buf).await?;
        self.writer.flush().await?;
        Ok(size)
    }
}
