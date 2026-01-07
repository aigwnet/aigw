use std::{collections::HashMap, net::SocketAddr, pin::Pin, sync::Arc, time::Duration};

use ::time::OffsetDateTime;
use aigw_core::{
    Algorithm, Buffer, CryptoCore, Frame, Shutdown, Signature, build_handshake_response,
    build_pong, parse_ack, parse_handshake_request, parse_ping,
};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, tcp::OwnedReadHalf},
    sync::{Mutex, RwLock, Semaphore, broadcast, mpsc},
    time,
};
use tracing::{debug, error, info};

use crate::{
    DatabaseClient,
    service::{find_cluster_by_name, save_ping, send_change_logs_to_aigw, update_or_insert_aigw},
};

use super::{Connections, connection::Connection};

/// Per-connection handler. Reads requests from `connection`
struct Handler {
    database_client: Arc<DatabaseClient>,
    addr: SocketAddr,
    reader: OwnedReadHalf,
    connection: Arc<Mutex<Connection>>,

    /// Connections
    connections: Connections,

    signatures: Arc<RwLock<HashMap<String, Arc<Signature>>>>,

    /// Listen for shutdown notifications.
    shutdown: Shutdown,

    /// Not used directly. Instead, when `Handler` is dropped...?
    _shutdown_complete: mpsc::Sender<()>,
}

impl Handler {
    async fn read(reader: &mut OwnedReadHalf, buffer: &mut Buffer) -> anyhow::Result<u8> {
        let data_type = reader.read_u8().await?;
        let data_len = reader.read_u32().await?;

        buffer.clear();
        buffer.set_len(data_len as usize);
        reader.read_exact(buffer).await?;
        Ok(data_type)
    }

    /// Process a single connection.
    async fn run(&mut self) -> anyhow::Result<()> {
        self.connections.insert(self.addr, self.connection.clone());
        let mut buffer = Buffer::new(128);

        // As long as the shutdown signal has not been received, try to read a
        // new request frame.
        while !self.shutdown.is_shutdown() {
            // While reading a request frame, also listen for the shutdown
            // signal.
            let data_type = tokio::select! {
                res = Handler::read(&mut self.reader, &mut buffer) => {
                    match res {
                        Ok(t) => t,
                        Err(e) => {
                            error!("Read error from {}: {:?}", self.addr, e);
                            break;
                        }
                    }
                }
                _ = self.shutdown.recv() => break,
            };

            match self.handle(data_type, &mut buffer).await {
                Ok(exit) => {
                    if exit {
                        break;
                    }
                }
                Err(e) => error!("Handle data error, {:?}", e),
            }
        }
        Ok(())
    }

    async fn handle(&self, data_type: u8, buffer: &mut Buffer) -> anyhow::Result<bool> {
        match data_type {
            Frame::HANDLESHAKE_REQ => {
                info!("Received handshake from: {}", &self.addr);

                let provider = move |cluster: &str| {
                    let database_client = self.database_client.clone();
                    let cluster = cluster.to_string();
                    let signatures = self.signatures.clone();

                    Box::pin(async move {
                        if let Some(r) = signatures.read().await.get(&cluster) {
                            return Ok(r.clone());
                        }

                        let c = find_cluster_by_name(&database_client.rb, &cluster).await?;
                        let signature = Arc::new(Signature::new(&c.security_key));
                        signatures.write().await.insert(cluster, signature.clone());
                        Ok(signature)
                    })
                        as Pin<Box<dyn Future<Output = anyhow::Result<Arc<Signature>>> + Send>>
                };

                let (handshake_request, signature) =
                    parse_handshake_request(&buffer[..], provider).await?;
                {
                    let algorithm = Algorithm::Aes256Gcm;
                    let (private_key, ecdh_public_key) = CryptoCore::create_ecdh_keypair();
                    let crypto = CryptoCore::new(
                        private_key,
                        &algorithm,
                        &handshake_request.public_key_data,
                    );
                    let mut connection = self.connection.lock().await;
                    connection.crypto = Some(crypto);
                    connection.cluster = Some(handshake_request.info.cluster.clone());
                    connection.ip = Some(handshake_request.info.ip.clone());
                    let buf =
                        build_handshake_response(&signature, &algorithm, ecdh_public_key.bytes())?;
                    connection.write(buf.as_ref()).await?;
                }

                // save current aigw
                if let Err(e) =
                    update_or_insert_aigw(&self.database_client.rb, handshake_request.info).await
                {
                    error!("Save aigw error: {:?}", e);
                }

                send_change_logs_to_aigw(
                    &self.connection,
                    &self.database_client.rb,
                    &handshake_request.log_points,
                )
                .await?;
            }
            Frame::HEARTBEAT_PING => {
                let mut log_points = vec![];
                {
                    let connection = &mut *self.connection.lock().await;
                    let crypto = &connection.crypto;
                    if let Some(crypto) = crypto {
                        let ping = parse_ping(buffer, crypto)?;
                        info!("Received ping from: {} {:?}", &self.addr, &ping);

                        log_points.extend(ping.log_points.clone());

                        let ts = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
                        build_pong(buffer, crypto, ts as i64)?;
                        connection.write(buffer.as_ref()).await?;

                        info!("Write pong to: {}", &self.addr);

                        if let Some(cluster) = &connection.cluster {
                            let ip = connection
                                .ip
                                .as_ref()
                                .map_or("".to_string(), |s| s.to_string());

                            save_ping(&self.database_client.rb, cluster.to_string(), ip, ping)
                                .await?;
                            info!("Saved heartbeat data: {}", &self.addr);
                        }
                    }
                }
                //
                send_change_logs_to_aigw(&self.connection, &self.database_client.rb, &log_points)
                    .await?;
            }

            Frame::ACK => {
                let connection = self.connection.lock().await;
                let crypto = &connection.crypto;
                if let Some(crypto) = crypto {
                    let ack = parse_ack(buffer, crypto)?;
                    debug!("Received ack frame: {}, {:?}", &self.addr, ack);
                }
            }

            Frame::CLOSE => {
                debug!("Received close frame: {}", &self.addr);
                return Ok(true);
            }

            _ => {
                return Err(anyhow::anyhow!("Protocol not supported."));
            }
        }

        Ok(false)
    }

    async fn cleanup(&mut self) {
        debug!("Cleaning connection {}", self.addr);

        // 1) close writer
        {
            let mut conn = self.connection.lock().await;
            let _ = conn.close().await;
        }

        // 2) remove from connections
        self.connections.remove(&self.addr);
    }
}

struct TcpServer {
    database_client: Arc<DatabaseClient>,
    /// TCP listener supplied by the `run` caller.
    listener: TcpListener,

    connections: Connections,

    /// Limit the max number of connections.
    limit_connections: Arc<Semaphore>,

    notify_shutdown: broadcast::Sender<()>,

    shutdown_complete_tx: mpsc::Sender<()>,
}

impl TcpServer {
    /// Run the server
    async fn run(&mut self) -> anyhow::Result<()> {
        info!("accepting inbound connections");

        loop {
            // Wait for a permit to become available
            let permit = self
                .limit_connections
                .clone()
                .acquire_owned()
                .await
                .unwrap();

            // Accept a new socket. This will attempt to perform error handling.
            // The `accept` method internally attempts to recover errors, so an
            // error here is non-recoverable.
            let (socket, addr) = self.accept().await?;
            let (reader, writer) = socket.into_split();

            let connection = Connection::new(writer);

            // Create the necessary per-connection handler state.
            let mut handler = Handler {
                database_client: self.database_client.clone(),
                addr,
                reader,
                // Initialize the connection state. This allocates read/write
                // buffers to perform redis protocol frame parsing.
                connection: Arc::new(Mutex::new(connection)),
                connections: self.connections.clone(),
                signatures: Arc::new(RwLock::new(HashMap::new())),
                // Receive shutdown notifications.
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),

                // Notifies the receiver half once all clones are
                // dropped.
                _shutdown_complete: self.shutdown_complete_tx.clone(),
            };

            debug!("New client connected: {}", addr);

            // Spawn a new task to process the connections. Tokio tasks are like
            // asynchronous green threads and are executed concurrently.
            tokio::spawn(async move {
                // Process the connection. If an error is encountered, log it.
                if let Err(err) = handler.run().await {
                    error!("Connection error: {:?}", err);
                }
                handler.cleanup().await;
                // Move the permit into the task and drop it after completion.
                // This returns the permit back to the semaphore.
                drop(permit);
            });
        }
    }

    /// Accept an inbound connection.
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

/// Run the tcp server.
pub async fn run(
    connections: Connections,
    database_client: Arc<DatabaseClient>,
    port: u16,
    max_connections: usize,
    shutdown: impl Future,
) {
    let addr: SocketAddr = ("[::]:".to_string() + port.to_string().as_str())
        .parse::<SocketAddr>()
        .unwrap();
    let r = TcpListener::bind(&addr).await;
    if let Ok(listener) = r {
        debug!("Listening on: {addr}");
        // When the provided `shutdown` future completes, we must send a shutdown
        // message to all active connections. We use a broadcast channel for this
        // purpose. The call below ignores the receiver of the broadcast pair, and when
        // a receiver is needed, the subscribe() method on the sender is used to create
        // one.
        let (notify_shutdown, _) = broadcast::channel(1);
        let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

        let mut server = TcpServer {
            database_client,
            listener,
            connections,
            limit_connections: Arc::new(Semaphore::new(max_connections)),
            notify_shutdown,
            shutdown_complete_tx,
        };

        // Concurrently run the server and listen for the `shutdown` signal. The
        // server task runs until an error is encountered, so under normal
        // circumstances, this `select!` statement runs until the `shutdown` signal
        // is received.
        //
        // `select!` statements are written in the form of:
        //
        // ```
        // <result of async op> = <async op> => <step to perform with result>
        // ```
        //
        // All `<async op>` statements are executed concurrently. Once the **first**
        // op completes, its associated `<step to perform with result>` is
        // performed.
        //
        // The `select!` macro is a foundational building block for writing
        // asynchronous Rust. See the API docs for more details:
        //
        // https://docs.rs/tokio/*/tokio/macro.select.html
        tokio::select! {
            res = server.run() => {
                // If an error is received here, accepting connections from the TCP
                // listener failed multiple times and the server is giving up and
                // shutting down.
                //
                // Errors encountered when handling individual connections do not
                // bubble up to this point.
                if let Err(err) = res {
                    error!( "failed to accept: {:?}", err);
                }
            }
            _ = shutdown => {
                // The shutdown signal has been received.
                info!("shutting down");
            }
        }

        // Extract the `shutdown_complete` receiver and transmitter
        // explicitly drop `shutdown_transmitter`. This is important, as the
        // `.await` below would otherwise never complete.
        let TcpServer {
            shutdown_complete_tx,
            notify_shutdown,
            ..
        } = server;

        // When `notify_shutdown` is dropped, all tasks which have `subscribe`d will
        // receive the shutdown signal and can exit
        drop(notify_shutdown);
        // Drop final `Sender` so the `Receiver` below can complete
        drop(shutdown_complete_tx);

        // Wait for all active connections to finish processing. As the `Sender`
        // handle held by the listener has been dropped above, the only remaining
        // `Sender` instances are held by connection handler tasks. When those drop,
        // the `mpsc` channel will close and `recv()` will return `None`.
        let _ = shutdown_complete_rx.recv().await;
    } else {
        error!("Bind error: {:?}", r.err());
    }
}
