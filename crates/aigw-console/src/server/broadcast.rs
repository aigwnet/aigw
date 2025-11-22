use std::{net::SocketAddr, sync::Arc, time::Duration};

use crate::{
    DatabaseClient,
    service::{send_change_log_to_peers, update_or_insert_local_peer},
};

use super::Connections;
use aigw_core::{Buffer, ChangeLog, DataFrame, LOCAL_IP, LogPoint, build_data};
use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::Receiver,
    time,
};
use tracing::{debug, error, info};

pub async fn broadcast(database_client: Arc<DatabaseClient>, mut receiver: Receiver<ChangeLog>) {
    while let Some(message) = receiver.recv().await {
        let _ = send_change_log_to_peers(&database_client.rb, message).await;
    }
}

pub async fn run(database_client: Arc<DatabaseClient>, connections: Connections, port: u16) {
    let addr: SocketAddr = ("[::]:".to_string() + port.to_string().as_str())
        .parse::<SocketAddr>()
        .unwrap();
    let r = TcpListener::bind(&addr).await;

    if let Ok(listener) = r {
        debug!("Listening broadcast on: {addr}");

        tokio::spawn(async move {
            update_local_peer_interval(database_client, port).await;
        });

        let mut server = BroadcastServer {
            connections,
            listener,
        };

        if let Err(e) = server.run().await {
            error!("Boradcast server run error, {:?}", e)
        }
    }
}

async fn update_local_peer_interval(database_client: Arc<DatabaseClient>, port: u16) {
    let host = &LOCAL_IP;
    loop {
        if let Err(e) = update_or_insert_local_peer(&database_client.rb, host, port).await {
            error!("update local peer error: {:?}", e);
        }
        tokio::time::sleep(Duration::from_millis(30_000)).await
    }
}

struct BroadcastServer {
    connections: Connections,
    listener: TcpListener,
}

impl BroadcastServer {
    /// Run the server
    async fn run(&mut self) -> anyhow::Result<()> {
        info!("Accepting inbound connections");

        loop {
            let (socket, addr) = self.accept().await?;
            if let Err(e) = self.handle(socket, addr).await {
                error!("Handle boradcast message error. {:?}", e);
            }
        }
    }

    async fn handle(&self, mut socket: TcpStream, addr: SocketAddr) -> anyhow::Result<()> {
        let capacity = 8 * 1024 * 1024;
        let length = socket.read_u32().await?;
        if length > 0 && length < capacity {
            let mut buffer = BytesMut::with_capacity(capacity as usize);
            unsafe {
                buffer.set_len(length as usize);
            }
            socket.read_exact(&mut buffer).await?;
            socket.write_u32(0).await?;

            let changelog: ChangeLog = ChangeLog::try_from(&buffer[..])?;
            debug!("Receive changelog form {:?}.", addr);
            self.broadcast_to_aigw(changelog).await?;
        }

        Ok(())
    }

    async fn broadcast_to_aigw(&self, changelog: ChangeLog) -> anyhow::Result<()> {
        let cluster = changelog.cluster.clone();
        // push the change to all aigw servers connected to this server.
        let aigw_servers: Vec<_> = self.connections.iter().map(|r| r.value().clone()).collect();
        let log_type = &changelog.log_type.clone();
        let log_action = &changelog.log_action.clone();
        let log_point = LogPoint {
            log_id: changelog.log_id,
            log_type: changelog.log_type,
        };
        let logs = vec![changelog];
        let data: DataFrame = DataFrame {
            logs,
            log_point: Some(log_point),
        };

        let mut buf = Buffer::new(128);

        for conn in aigw_servers {
            let mut conn = conn.lock().await;
            if let Some(c) = &conn.cluster
                && c.eq(&cluster)
                && let Some(crypto) = &conn.crypto
            {
                build_data(&mut buf, data.clone(), crypto)?;

                //
                if let Err(e) = conn.write(buf.as_ref()).await {
                    error!("Wrtie to aigw error, {:?}", e);
                }

                info!(target: "broadcast", "Send [{:?} {:?}] message to {} {}.", log_type, log_action, &cluster, conn.ip.as_ref().map_or("", |s|s));
            }
        }

        Ok(())
    }

    async fn accept(&mut self) -> anyhow::Result<(TcpStream, SocketAddr)> {
        let mut backoff = 1;

        // Try to accept a few times
        loop {
            // Perform the accept operation. If a socket is successfully
            // accepted, return it. Otherwise, save the error.
            match self.listener.accept().await {
                Ok((socket, addr)) => return Ok((socket, addr)),
                Err(err) => {
                    if backoff > 64 {
                        // Accept has failed too many times. Return the error.
                        return Err(err.into());
                    }
                }
            }

            // Pause execution until the back off period elapses.
            time::sleep(Duration::from_secs(backoff)).await;

            // Double the back off
            backoff *= 2;
        }
    }
}
