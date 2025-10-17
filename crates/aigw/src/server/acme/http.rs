use std::sync::Arc;

use aigw_core::AcmeToken;

use crate::server::Storage;

pub struct Http01Handler {
    pub(crate) storage: Arc<Storage>,
}

impl Http01Handler {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    pub fn handle(&self, host: &str, token: &str) -> Option<AcmeToken> {
        self.storage.find_token(host, token)
    }
}
