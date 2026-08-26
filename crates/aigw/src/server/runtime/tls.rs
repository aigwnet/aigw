use std::{any::Any, sync::Arc};

use aigw_core::{DynamicCert, Site, TlsPrivateKey};
use async_trait::async_trait;
use pingora_core::{
    listeners::TlsAccept,
    protocols::tls::TlsRef,
    tls::{pkey::PKey, ssl, ssl_sys::SSL_get_ex_data, x509::X509},
};
use tracing::error;

use crate::server::{runtime::fingerprint::JA4_INDEX, storage::Storage};

pub struct DynamicTlsAccept {
    storage: Arc<Storage>,
}

impl DynamicTlsAccept {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    fn set_dynamic_cert(&self, site: &Site, ssl: &mut TlsRef) -> anyhow::Result<()> {
        let cert = site
            .tls_cert
            .as_ref()
            .ok_or(anyhow::anyhow!("Cert not found"))?;
        let key = site
            .tls_private_key
            .as_ref()
            .ok_or(anyhow::anyhow!("Cert private key not found"))?;
        self.use_dynamic_cert(key, cert, ssl)
    }

    fn use_dynamic_cert(
        &self,
        key: &TlsPrivateKey,
        cert: &DynamicCert,
        ssl: &mut TlsRef,
    ) -> anyhow::Result<()> {
        for cert in &cert.cert_chain {
            let cert = X509::from_der(cert.cert())?;
            pingora_core::tls::ext::ssl_add_chain_cert(ssl, &cert)?;
        }
        let cert = X509::from_der(cert.cert.cert())?;
        pingora_core::tls::ext::ssl_use_certificate(ssl, &cert)?;
        let key = PKey::private_key_from_der(key.0.secret_der())?;
        pingora_core::tls::ext::ssl_use_private_key(ssl, &key)?;
        Ok(())
    }
}

#[async_trait]
impl TlsAccept for DynamicTlsAccept {
    async fn certificate_callback(&self, ssl: &mut TlsRef) -> () {
        self.storage.tls();
        // NOTE: pingora's certificate_callback cannot return an error, and the
        // acceptor has no default certificate. Rejection therefore works by
        // leaving no certificate on this (per-connection) SSL object, which
        // makes resume_accept() abort the handshake. Do NOT load any fallback
        // certificate here, or unknown-SNI connections would silently succeed.
        if let Some(sni) = ssl.servername(ssl::NameType::HOST_NAME) {
            let site = self
                .storage
                .find_site(sni)
                .map_or(self.storage.find_default_tls_site(), Some);
            if let Some(site) = site {
                if let Err(e) = self.set_dynamic_cert(&site, ssl) {
                    error!("Add cert error, {:?}", e);
                    self.storage.error();
                }
            } else {
                error!("Site  {:?} not found.", sni);
                self.storage.error();
            }
        } else if let Some(site) = self.storage.find_default_tls_site() {
            if let Err(e) = self.set_dynamic_cert(&site, ssl) {
                error!("Add cert error, {:?}", e);
                self.storage.error();
            }
        } else {
            error!("Unknown HTTPS request without SNI.");
            self.storage.error();
        }
    }

    async fn handshake_complete_callback(
        &self,
        ssl: &TlsRef,
    ) -> Option<Arc<dyn Any + Send + Sync>> {
        use foreign_types::ForeignTypeRef;

        unsafe {
            let ptr = SSL_get_ex_data(ssl.as_ptr(), *JA4_INDEX);
            if !ptr.is_null() {
                let boxed: Box<(String, String)> = Box::from_raw(ptr as *mut _);
                let (ja4_hash, _) = *boxed;

                let fp = FingerPrint { ja4_hash };

                Some(Arc::new(fp))
            } else {
                None
            }
        }
    }
}

pub(crate) struct FingerPrint {
    pub ja4_hash: String,
}
