use std::sync::Arc;

use aigw_core::Site;
use async_trait::async_trait;
use log::error;
use pingora_core::{listeners::TlsAccept, protocols::tls::TlsRef, tls::ssl};

use crate::server::storage::Storage;

pub struct DynamicTlsAccept {
    storage: Arc<Storage>,
}

impl DynamicTlsAccept {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    fn add_dynamic_cert(&self, site: &Site, ssl: &mut TlsRef) -> anyhow::Result<()> {
        let cert = site
            .tls_cert
            .as_ref()
            .ok_or(anyhow::anyhow!("Cert not found"))?;
        let key = site
            .tls_private_key
            .as_ref()
            .ok_or(anyhow::anyhow!("Cert private key not found"))?;
        for cert in &cert.cert_chain {
            pingora_core::tls::ext::ssl_add_chain_cert(ssl, cert)?;
        }
        pingora_core::tls::ext::ssl_use_certificate(ssl, &cert.cert)?;
        pingora_core::tls::ext::ssl_use_private_key(ssl, key.as_ref())?;
        Ok(())
    }
}

#[async_trait]
impl TlsAccept for DynamicTlsAccept {
    async fn certificate_callback(&self, ssl: &mut TlsRef) -> () {
        self.storage.tls();
        if let Some(sni) = ssl.servername(ssl::NameType::HOST_NAME) {
            if let Some(site) = self.storage.find_site(sni) {
                if let Err(e) = self.add_dynamic_cert(&site, ssl) {
                    error!("Add cert error, {:?}", e);
                    ssl.set_verify(ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT);
                }
            } else {
                error!("Site  {:?} not found.", sni);
                ssl.set_verify(ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT);
                self.storage.error();
            }
        } else {
            error!("Unknowns HTTPS request without SNI.");
            ssl.set_verify(ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT);
            self.storage.error();
        }
    }
}
