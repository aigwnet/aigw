use std::{collections::HashMap, net::ToSocketAddrs, sync::Arc, time::Duration};

use aigw_core::{
    Buffer, Close, CryptoCore, DataAck, Frame, HandshakeInfo, LOCAL_IP, LOGGER_TIME_FORMAT,
    Signature, build_ack, build_close, build_handshake_request, build_ping,
    date_format_local_nanos, parse_data, parse_handshake_response, parse_pong, statistics,
};
use bytes::BytesMut;
use sysinfo::System;
use time::OffsetDateTime;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::{RwLock, broadcast::Sender, mpsc},
    time::Instant,
};
use tracing::{error, info};

use crate::{
    server::{AigwConfig, storage::Storage},
    version::VERSION,
};

use super::DataFrameHandler;

const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

pub struct ConsoleClient {
    shutdown_tx: Arc<Sender<()>>,
    address: String,
    cluster: String,
    crypto: Arc<RwLock<Option<CryptoCore>>>,
    signature: Arc<Signature>,
}

impl ConsoleClient {
    pub fn new(config: Arc<AigwConfig>, shutdown_tx: Arc<Sender<()>>) -> Self {
        let signature = Arc::new(Signature::new(config.console().key()));
        let crypto = Arc::new(RwLock::new(None));

        Self {
            shutdown_tx,
            address: config.console().address().to_owned(),
            cluster: config.console().cluster().to_owned(),
            signature,
            crypto,
        }
    }

    pub async fn start(&self, data_handler: Arc<DataFrameHandler>) {
        let addr = &self.address;
        let signature = &self.signature;
        loop {
            let (tx, rx) = mpsc::channel::<Vec<u8>>(1024);
            let sender = Arc::new(tx);

            let socket_addrs = match addr.to_socket_addrs() {
                Ok(addrs) => addrs.collect::<Vec<_>>(),
                Err(e) => {
                    error!("Failed to resolve address {}: {}", addr, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            if socket_addrs.is_empty() {
                error!("Failed to resolve address {}", addr);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            match TcpStream::connect(socket_addrs[0]).await {
                Ok(stream) => {
                    let r = ConsoleClient::run(
                        data_handler.clone(),
                        self.shutdown_tx.clone(),
                        sender.clone(),
                        rx,
                        stream,
                        addr,
                        signature,
                        self.crypto.clone(),
                        self.cluster.clone(),
                    )
                    .await;
                    match r {
                        Ok(exit) => {
                            if exit {
                                let _ = self.close(&sender).await;
                                break;
                            }
                        }
                        Err(e) => {
                            error!(
                                "Run error: {}. Retrying in {}s...",
                                e,
                                RECONNECT_DELAY.as_secs()
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Connection failed: {}. Retrying in {}s...",
                        e,
                        RECONNECT_DELAY.as_secs()
                    );
                    tokio::time::sleep(RECONNECT_DELAY).await;
                }
            }
        }
        info!(target:"console", "Aigw Console client exited.")
    }

    #[allow(clippy::too_many_arguments)]
    async fn run(
        data_handler: Arc<DataFrameHandler>,
        shutdown_tx: Arc<Sender<()>>,
        sender: Arc<mpsc::Sender<Vec<u8>>>,
        rx: mpsc::Receiver<Vec<u8>>,
        stream: TcpStream,
        addr: &str,
        signature: &Signature,
        crypto: Arc<RwLock<Option<CryptoCore>>>,
        cluster: String,
    ) -> anyhow::Result<bool> {
        info!(target:"console", "Connected to {}", addr);
        let (mut reader, mut writer) = stream.into_split();

        let log_points = data_handler.storage.load_log_points().await?;
        info!(target:"console", "Send handshake, log_points: {:?}", log_points);
        // start handshake
        let (private_key, ecdh_public_key) = CryptoCore::create_ecdh_keypair();

        let mut sys = System::new_all();

        // First we update all information of our `System` struct.
        sys.refresh_all();

        let info = os_info::get();
        let mut os = info.to_string();
        if let Some(r) = info.architecture() {
            os += "/";
            os += r;
        }
        if let Some(e) = info.edition() {
            os += " (";
            os += e;
            os += ")";
        }

        let info = HandshakeInfo {
            ip: LOCAL_IP.clone(),
            cluster,
            version: VERSION.to_string(),
            os_name: info.os_type().to_string(),
            os_version: info.version().to_string(),
            os_arch: info.architecture().map_or("".to_string(), |s| s.to_owned()),
            cpu_name: sys.cpus()[0].brand().to_string(),
            cpu_vendor: sys.cpus()[0].vendor_id().to_string(),
            cpu_frequency: sys.cpus()[0].frequency(),
            cpu_nums: sys.cpus().len() as u32,
        };

        let buffer = build_handshake_request(signature, ecdh_public_key.bytes(), log_points, info)?;
        writer.write_all(&buffer).await?;
        writer.flush().await?;

        let data_type = reader.read_u8().await?;
        if data_type != Frame::HANDLESHAKE_RSP {
            return Err(anyhow::anyhow!("handshake error."));
        }

        let length = reader.read_u32().await?;
        if length as usize > 65535 {
            return Err(anyhow::anyhow!("Handshake response too large: {length}"));
        }
        let mut buf = BytesMut::with_capacity(65535);
        buf.resize(length as usize, 0);
        reader.read_exact(&mut buf).await?;

        let response = parse_handshake_response(&buf, signature)?;
        let new_crypto =
            CryptoCore::new(private_key, &response.algorithm, &response.public_key_data);
        {
            *crypto.write().await = Some(new_crypto);
        }
        info!(target:"console", "Handshake successfully to {}", addr);

        let crypto_for_hb = crypto.clone();
        let sender_for_hb = sender.clone();
        let storage = data_handler.storage.clone();
        let mut heartbeat_handle = tokio::spawn(ConsoleClient::spawn_heartbeat(
            storage,
            sender_for_hb,
            crypto_for_hb,
        ));

        let mut send_handle = tokio::spawn(ConsoleClient::spawn_send_task(writer, rx));

        let crypto_for_r = crypto.clone();
        let data_handler_for_r = data_handler.clone();
        let sender_for_r = sender.clone();
        let mut recv_handle = tokio::spawn(ConsoleClient::spawn_receive_task(
            sender_for_r,
            data_handler_for_r,
            reader,
            crypto_for_r,
        ));

        let mut shutdown = shutdown_tx.subscribe();
        let r = tokio::select! {
            _ = &mut send_handle => {
                info!(target:"console", "Send task exited");
                false
            },
            _ = &mut recv_handle => {
                info!(target:"console", "Receive task exited");
                false
            },
            _ = &mut heartbeat_handle => {
                info!(target:"console", "Heartbeat task exited");
                false
            },
            _ = shutdown.recv() => {
                 info!(target:"console", "Shutting down aigw console client.");
                 true
            }
        };

        send_handle.abort();
        recv_handle.abort();
        heartbeat_handle.abort();

        Ok(r)
    }

    async fn spawn_heartbeat(
        storage: Arc<Storage>,
        tx: Arc<mpsc::Sender<Vec<u8>>>,
        crypto: Arc<RwLock<Option<CryptoCore>>>,
    ) {
        if let Ok(log_points) = storage.load_log_points().await
            && log_points.is_empty()
        {
            tokio::time::sleep(Duration::from_secs(30)).await;
        }

        let mut interval =
            tokio::time::interval_at(Instant::now() + Duration::from_secs(5), HEARTBEAT_INTERVAL);
        let mut buffer = Buffer::new(32);
        loop {
            interval.tick().await;

            let crypto = &*crypto.read().await;
            if let Some(crypto) = crypto {
                let log_points = storage.load_log_points().await.map_or(vec![], |v| v);
                let now = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
                let ts = now as i64;
                let ts_str = date_format_local_nanos((ts as i128) * 1_000_000, LOGGER_TIME_FORMAT)
                    .unwrap_or_default();
                info!(target:"console", "Ping ==> {:?} log_points: {:?}", ts_str, log_points);
                let pv = storage.pv_swap();
                let rt = if pv == 0 { 0 } else { storage.rt_swap() / pv };

                let mut map = HashMap::new();
                let mut http_code = HashMap::new();
                http_code.insert("1xx".to_string(), storage.http_code_1xx_swap());
                http_code.insert("2xx".to_string(), storage.http_code_2xx_swap());
                http_code.insert("3xx".to_string(), storage.http_code_3xx_swap());
                http_code.insert("4xx".to_string(), storage.http_code_4xx_swap());
                http_code.insert("5xx".to_string(), storage.http_code_5xx_swap());

                let mut http_source = HashMap::new();
                http_source.insert("Pc".to_string(), storage.http_source_pc_swap());
                http_source.insert("Pad".to_string(), storage.http_source_pad_swap());
                http_source.insert("Mobile".to_string(), storage.http_source_mobile_swap());
                http_source.insert("Bot".to_string(), storage.http_source_bot_swap());
                http_source.insert("Unknown".to_string(), storage.http_source_unknown_swap());

                let http_country = storage.countries();

                map.insert("http_code", http_code);
                map.insert("http_source", http_source);
                map.insert("http_country", http_country);
                let ext_info = serde_json::to_string(&map).map_or("{}".to_string(), |s| s);

                let statistics =
                    statistics(storage.tls_swap(), pv, rt, storage.error_swap(), ext_info).await;
                match statistics {
                    Ok(statistics) => {
                        if let Ok(()) = build_ping(&mut buffer, crypto, ts, log_points, statistics)
                        {
                            let _ = tx.send(buffer[..].to_vec()).await;
                        }
                    }
                    Err(err) => {
                        error!("Statistics Error: {:?}", err);
                    }
                }
            }
        }
    }

    async fn spawn_send_task(mut writer: OwnedWriteHalf, mut rx: mpsc::Receiver<Vec<u8>>) {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = writer.write_all(&msg).await {
                error!("Failed to send message: {}", e);
                break;
            }
        }
    }

    async fn spawn_receive_task(
        sender: Arc<mpsc::Sender<Vec<u8>>>,
        data_handler: Arc<DataFrameHandler>,
        mut reader: OwnedReadHalf,
        crypto: Arc<RwLock<Option<CryptoCore>>>,
    ) -> anyhow::Result<()> {
        let mut buffer = Buffer::new(32);
        loop {
            let data_type = reader.read_u8().await?;
            let data_length = reader.read_u32().await?;
            buffer.set_len(data_length as usize)?;
            match reader.read_exact(buffer.message_mut()).await {
                Ok(0) => {
                    error!("Server disconnected");
                    break;
                }
                Ok(_n) => {
                    let crypto = &*crypto.read().await;
                    if let Some(crypto) = crypto
                        && let Err(e) = ConsoleClient::handle(
                            &sender,
                            &data_handler,
                            data_type,
                            &mut buffer,
                            crypto,
                        )
                        .await
                    {
                        error!("Handle data error, {:?}", e);
                    }

                    buffer.clear();
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle(
        sender: &mpsc::Sender<Vec<u8>>,
        data_handler: &DataFrameHandler,
        data_type: u8,
        buffer: &mut Buffer,
        crypto: &CryptoCore,
    ) -> anyhow::Result<()> {
        match data_type {
            Frame::HEARTBEAT_PONG => {
                let pong = parse_pong(buffer, crypto)?;

                let ts = date_format_local_nanos((pong.ts as i128) * 1_000_000, LOGGER_TIME_FORMAT);
                info!(target:"console", "Pong <== {:?}", ts.unwrap_or_default());
            }
            Frame::DATA => {
                let data = parse_data(buffer, crypto)?;
                data_handler.handle(&data).await?;

                if let Some(log_point) = data.log_point {
                    build_ack(
                        buffer,
                        DataAck {
                            log_point: Some(log_point),
                        },
                        crypto,
                    )?;
                    if let Ok(()) = sender.send(buffer.as_ref().to_vec()).await {
                        info!(target:"console", "Send ack: {}, {:?}", log_point.log_id, log_point.log_type);
                    }
                }
            }
            _ => {
                error!("{:?} not supported.", data_type);
            }
        }
        Ok(())
    }

    pub async fn close(&self, sender: &mpsc::Sender<Vec<u8>>) -> anyhow::Result<()> {
        let crypto = &*self.crypto.read().await;
        if let Some(crypto) = crypto {
            let mut buffer = Buffer::new(32);
            build_close(&mut buffer, &Close {}, crypto)?;
            sender.send(buffer.as_ref().to_vec()).await?;
        }
        info!(target:"console", "Send close to aigw console server.");
        Ok(())
    }
}
