use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use connection::Connection;
use tokio::sync::Mutex;

pub(crate) mod broadcast;
pub(crate) mod connection;
pub(crate) mod http;
pub(crate) mod tcp;

pub(crate) type Connections = Arc<Mutex<HashMap<SocketAddr, Arc<Mutex<Connection>>>>>;

#[cfg(test)]
mod tests {
    use std::{io::Write, thread::sleep, time::Duration};

    use bytes::{BufMut, BytesMut};

    #[test]
    pub fn test() -> anyhow::Result<()> {
        let mut stream = std::net::TcpStream::connect("127.0.0.1:9527")?;
        let mut buf = BytesMut::with_capacity(8 * 1024 * 1024);
        buf.put_u16(12);
        buf.put_u32(8192);
        for _ in 0..8192 {
            buf.put_u8(1);
        }
        stream.write_all(&buf)?;
        stream.flush()?;

        let _ = sleep(Duration::from_secs(30));
        Ok(())
    }
}
