mod client;
mod handler;

use std::sync::Arc;

use async_trait::async_trait;
use client::ConsoleClient;
use handler::DataFrameHandler;
use pingora_core::{
    server::{ListenFds, ShutdownWatch},
    services::Service,
};
use tokio::sync::{
    Mutex,
    mpsc::{self, Receiver},
};

use crate::server::{AigwConfig, Storage};

pub struct AigwConsoleService {
    storage: Arc<Storage>,
    console_client: Arc<ConsoleClient>,
    rx: Arc<Mutex<Receiver<Vec<u8>>>>,
    #[cfg(target_os = "linux")]
    epbf_config: EpbfConfig,
}

impl AigwConsoleService {
    pub fn new(
        config: Arc<AigwConfig>,
        storage: Arc<Storage>,
        shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
        #[cfg(target_os = "linux")] epbf_config: EpbfConfig,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(1024);
        let tx = Arc::new(tx);
        let rx = Arc::new(Mutex::new(rx));

        let console_client = Arc::new(ConsoleClient::new(
            config.clone(),
            shutdown_tx.clone(),
            tx,
            #[cfg(target_os = "linux")]
            ebpf,
        ));

        Self {
            storage,
            console_client,
            rx,
            #[cfg(target_os = "linux")]
            epbf_config,
        }
    }
}

#[async_trait]
impl Service for AigwConsoleService {
    async fn start_service(
        &mut self,
        #[cfg(unix)] _fds: Option<ListenFds>,
        mut _shutdown: ShutdownWatch,
        _listeners_per_fd: usize,
    ) {
        #[cfg(target_os = "linux")]
        let ebpf_handler = epbf::run(&self.epbf_config);
        let data_handler = Arc::new(DataFrameHandler::new(
            self.storage.clone(),
            #[cfg(target_os = "linux")]
            ebpf_handler,
        ));

        self.console_client.start(self.rx.clone(), data_handler).await;
    }

    /// The name of the service, just for logging and naming the threads assigned to this service
    ///
    /// Note that due to the limit of the underlying system, only the first 16 chars will be used
    fn name(&self) -> &str {
        "aigwc"
    }

    /// The preferred number of threads to run this service
    ///
    /// If `None`, the global setting will be used
    fn threads(&self) -> Option<usize> {
        Some(2)
    }
}
