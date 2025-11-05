use std::{fs, sync::Arc};

use aigw_core::{AcmeToken, Cluster, DataFrame, LogAction, LogType, Site};
use tracing::info;

use crate::server::storage::Storage;

pub struct DataFrameHandler {
    pub(crate) storage: Arc<Storage>,
}

impl DataFrameHandler {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
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
                    info!("{:?} cluster: {:?}", change_log.log_action, cluster.name);
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
                        info!("{:?} site: {:?}", change_log.log_action, site.name);
                        self.storage.add_site(Arc::new(site));
                    }
                    LogAction::Delete => {
                        let site: Site = serde_json::from_slice(&change_log.data)?;
                        let mut path = self.storage.data_dir.clone();
                        path.push("site");
                        path.push(site.name.clone() + ".json");
                        info!("Remove site: {:?}", site.name);
                        let _ = fs::remove_file(path);
                        self.storage.remove_site(&site);
                    }
                },
                LogType::Acme => {
                    let acme_token: AcmeToken = serde_json::from_slice(&change_log.data)?;
                    match change_log.log_action {
                        LogAction::Add | LogAction::Update => {
                            info!(
                                "Add acme token: {},{},{}",
                                acme_token.host, acme_token.token, acme_token.proof
                            );
                            self.storage.add_token(acme_token);
                        }
                        LogAction::Delete => {
                            info!(
                                "Remove acme token: {},{}",
                                acme_token.host, acme_token.token
                            );
                            self.storage
                                .remove_token(&acme_token.host, &acme_token.token);
                        }
                    }
                    return Ok(true);
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
