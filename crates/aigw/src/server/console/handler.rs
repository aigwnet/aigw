use std::{fs, sync::Arc};

use aigw_core::{AcmeToken, Cluster, DataFrame, LogAction, LogType, Site};
use tracing::info;

use crate::server::storage::Storage;

pub struct DataFrameHandler {
    pub(crate) storage: Arc<Storage>,
    #[cfg(target_os = "linux")]
    pub(crate) ebpf_handler: Arc<crate::server::epbf::EbpfHandler>,
}

impl DataFrameHandler {
    pub fn new(
        storage: Arc<Storage>,
        #[cfg(target_os = "linux")] ebpf_handler: Arc<crate::server::epbf::EbpfHandler>,
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
                        fs::create_dir_all(&path)?;
                    }
                    path.push("cluster.json");

                    let cluster_str = serde_json::to_string(&cluster)?;
                    fs::write(path, cluster_str)?;
                    info!(target: "console", "{:?} cluster: {:?}", change_log.log_action, cluster.name);

                    #[cfg(target_os = "linux")]
                    {
                        self.ebpf_handler
                            .handle_switch(cluster.enable_white_list, cluster.enable_block_list)?;
                    }
                    self.storage.store_cluster(Arc::new(cluster));
                }
                LogType::Site => match change_log.log_action {
                    LogAction::Add | LogAction::Update => {
                        let site: Site = serde_json::from_slice(&change_log.data)?;
                        let mut path = self.storage.data_dir.clone();
                        path.push("site");
                        if !path.exists() {
                            fs::create_dir_all(&path)?;
                        }
                        path.push(site.name.clone() + ".json");
                        fs::write(path, serde_json::to_string_pretty(&site)?)?;
                        info!(target: "console", "{:?} site: {:?}", change_log.log_action, site.name);
                        self.storage.add_site(Arc::new(site));
                    }
                    LogAction::Delete => {
                        let site: Site = serde_json::from_slice(&change_log.data)?;
                        let mut path = self.storage.data_dir.clone();
                        path.push("site");
                        path.push(site.name.clone() + ".json");
                        info!(target: "console", "Remove site: {:?}", site.name);
                        let _ = fs::remove_file(path);
                        self.storage.remove_site(&site);
                    }
                },
                LogType::Acme => {
                    let acme_token: AcmeToken = serde_json::from_slice(&change_log.data)?;
                    match change_log.log_action {
                        LogAction::Add | LogAction::Update => {
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
                                use aigw_core::IpUpdateList;

                                let ip_list_for_update: IpUpdateList =
                                    serde_json::from_slice(&change_log.data)?;
                                self.ebpf_handler.handle_update(ip_list_for_update)?;
                            }
                            LogAction::Delete => {
                                use aigw_core::IpDeleteList;

                                let ip_list_for_delete: IpDeleteList =
                                    serde_json::from_slice(&change_log.data)?;
                                self.ebpf_handler.handle_delete(ip_list_for_delete)?;
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
