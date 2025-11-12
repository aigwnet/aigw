mod client;
#[cfg(target_os = "linux")]
mod epbf;
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

use crate::server::Storage;

pub struct AigwConsoleService {
    console_client: Arc<ConsoleClient>,
    rx: Arc<Mutex<Receiver<Vec<u8>>>>,
    #[cfg(target_os = "linux")]
    ifcae: String,
    #[cfg(target_os = "linux")]
    ebpf: Option<String>,
}

impl AigwConsoleService {
    pub fn new(
        storage: Arc<Storage>,
        shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
        address: &str,
        password: &str,
        cluster: String,
        #[cfg(target_os = "linux")] ifcae: String,
        #[cfg(target_os = "linux")] ebpf: Option<String>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(1024);
        let tx = Arc::new(tx);
        let rx = Arc::new(Mutex::new(rx));

        let console_client = Arc::new(ConsoleClient::new(
            storage.clone(),
            shutdown_tx.clone(),
            address,
            password,
            cluster,
            tx,
        ));

        Self {
            console_client,
            rx,
            #[cfg(target_os = "linux")]
            ifcae,
            #[cfg(target_os = "linux")]
            ebpf,
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
        let _e = epbf::run(&self.ifcae, self.ebpf.as_ref());
        self.console_client.start(self.rx.clone()).await;
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
