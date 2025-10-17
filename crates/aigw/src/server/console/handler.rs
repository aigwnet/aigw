use std::{fs, sync::Arc};

use aigw_core::{AcmeToken, DataFrame, LogAction, LogType, Site};
use log::info;

use crate::server::storage::Storage;

pub struct DataFramHandler {
    pub(crate) storage: Arc<Storage>,
}

impl DataFramHandler {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub async fn handle(&self, item: &DataFrame) -> anyhow::Result<bool> {
        //
        for change_log in &item.logs {
            //
            match change_log.log_type {
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
                        let server = Arc::new(site);
                        info!("Add new site: {:?}", server.name);
                        self.storage.add_site(server);
                    }
                    LogAction::Delete => {
                        let site: Site = serde_json::from_slice(&change_log.data)?;
                        let mut path = self.storage.data_dir.clone();
                        path.push("site");
                        if !path.exists() {
                            fs::create_dir_all(&path)?;
                        }
                        path.push(site.name.clone() + ".json");
                        info!("Remove site: {:?}", site.name);
                        let _ = fs::remove_file(path);
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
