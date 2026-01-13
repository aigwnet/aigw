use std::sync::Arc;

use aigw_core::{AcmeToken, Cluster, DataFrame, LogAction, LogType, Site};
use tokio::fs;
use tracing::info;

use crate::server::storage::Storage;

pub struct DataFrameHandler {
    pub(crate) storage: Arc<Storage>,
    #[cfg(target_os = "linux")]
    pub(crate) ebpf_handler: Option<Arc<crate::server::epbf::EbpfHandler>>,
}

impl DataFrameHandler {
    pub fn new(
        storage: Arc<Storage>,
        #[cfg(target_os = "linux")] ebpf_handler: Option<Arc<crate::server::epbf::EbpfHandler>>,
    ) -> Self {
        Self {
            storage,
            #[cfg(target_os = "linux")]
            ebpf_handler,
        }
    }

    pub async fn handle(&self, item: &DataFrame) -> anyhow::Result<bool> {
        //
        for change_log in &item.logs {
            //
            match change_log.log_type {
                LogType::Cluster => {
                    let cluster: Cluster = serde_json::from_slice(&change_log.data)?;

                    let mut path = self.storage.data_dir.clone();
                    if !path.exists() {
                        fs::create_dir_all(&path).await?;
                    }
                    path.push("cluster.json");

                    let cluster_str = serde_json::to_string_pretty(&cluster)?;
                    fs::write(path, &cluster_str).await?;
                    info!(target: "console", "{:?} cluster: {} ==> {}", change_log.log_action, cluster.name, &cluster_str);

                    #[cfg(target_os = "linux")]
                    {
                        if let Some(ebpf_handler) = &self.ebpf_handler {
                            ebpf_handler
                                .handle_switch(cluster.enable_white_list, cluster.enable_block_list)
                                .await?;
                        }
                    }
                    self.storage.store_cluster(Arc::new(cluster));
                }
                LogType::Site => match change_log.log_action {
                    LogAction::Create | LogAction::Update => {
                        let site: Site = serde_json::from_slice(&change_log.data)?;
                        let mut path = self.storage.data_dir.clone();
                        path.push("site");
                        if !path.exists() {
                            fs::create_dir_all(&path).await?;
                        }
                        path.push(site.name.clone() + ".json");
                        fs::write(path, serde_json::to_string_pretty(&site)?).await?;
                        info!(target: "console", "{:?} site: {:?}", change_log.log_action, site.name);
                        self.storage.add_site(site)?;
                    }
                    LogAction::Delete => {
                        let site: Site = serde_json::from_slice(&change_log.data)?;
                        let mut path = self.storage.data_dir.clone();
                        path.push("site");
                        path.push(site.name.clone() + ".json");
                        info!(target: "console", "Remove site: {:?}", site.name);
                        let _ = fs::remove_file(path).await;
                        self.storage.remove_site(&site);
                    }
                },
                LogType::Acme => {
                    let acme_token: AcmeToken = serde_json::from_slice(&change_log.data)?;
                    match change_log.log_action {
                        LogAction::Create | LogAction::Update => {
                            info!(target: "console",
                                "Add acme token: {},{},{}",
                                acme_token.host, acme_token.token, acme_token.proof
                            );
                            self.storage.add_token(acme_token);
                        }
                        LogAction::Delete => {
                            info!(target: "console",
                                "Remove acme token: {},{}",
                                acme_token.host, acme_token.token
                            );
                            self.storage
                                .remove_token(&acme_token.host, &acme_token.token);
                        }
                    }
                    return Ok(true);
                }
                LogType::IpLayer4 => {
                    #[cfg(target_os = "linux")]
                    {
                        match change_log.log_action {
                            LogAction::Add | LogAction::Update => {
                                let ip_list_for_update: aigw_core::IpList =
                                    serde_json::from_slice(&change_log.data)?;
                                self.storage.add_ip_cidr(&ip_list_for_update).await?;
                                if let Some(ebpf_handler) = &self.ebpf_handler {
                                    ebpf_handler.handle_update(ip_list_for_update).await?;
                                }
                            }
                            LogAction::Delete => {
                                let ip_list_for_delete: aigw_core::IpList =
                                    serde_json::from_slice(&change_log.data)?;
                                self.storage.remove_ip_cidr(&ip_list_for_delete).await?;
                                if let Some(ebpf_handler) = &self.ebpf_handler {
                                    ebpf_handler.handle_delete(ip_list_for_delete).await?;
                                }
                            }
                        }
                    }
                }
            }
        }

        //
        if let Some(log_point) = item.log_point {
            self.storage
                .update_log_point(log_point.log_type.code(), log_point.log_id)
                .await?;
        }
        Ok(true)
    }
}
