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

use crate::server::{AigwConfig, Storage};

pub struct AigwConsoleService {
    #[cfg(target_os = "linux")]
    config: Arc<AigwConfig>,
    storage: Arc<Storage>,
    console_client: Arc<ConsoleClient>,
    #[cfg(target_os = "linux")]
    epbf_config: super::epbf::EpbfConfig,
}

impl AigwConsoleService {
    pub fn new(
        config: Arc<AigwConfig>,
        storage: Arc<Storage>,
        shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
        #[cfg(target_os = "linux")] epbf_config: super::epbf::EpbfConfig,
    ) -> Self {
        let console_client = Arc::new(ConsoleClient::new(config.clone(), shutdown_tx.clone()));

        Self {
            #[cfg(target_os = "linux")]
            config,
            storage,
            console_client,
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
        let ebpf_handler = super::epbf::run(&self.epbf_config, self.config.console().address())
            .ok()
            .map(Arc::new);

        #[cfg(target_os = "linux")]
        if let Some(ebpf_handler) = &ebpf_handler {
            if let Ok(data) = self.storage.load_ip_cidr(1).await {
                let ip_list_for_update = aigw_core::IpList { item_type: 1, data };
                let _ = ebpf_handler.handle_update(ip_list_for_update).await;
            }
            if let Ok(data) = self.storage.load_ip_cidr(2).await {
                let ip_list_for_update = aigw_core::IpList { item_type: 2, data };
                let _ = ebpf_handler.handle_update(ip_list_for_update).await;
            }
        }
        let data_handler = Arc::new(DataFrameHandler::new(
            self.storage.clone(),
            #[cfg(target_os = "linux")]
            ebpf_handler,
        ));

        self.console_client.start(data_handler).await;
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
