use std::{sync::Arc, time::Duration};

use boring::{
    hash::MessageDigest,
    pkey::{PKey, Private},
    stack::Stack,
    x509::{X509Name, X509Req, extension::SubjectAlternativeName},
};
use serde::Deserialize;
use serde_json::json;
use tracing::debug;

use crate::service::acme::{
    account::Account,
    error::{Error, ServerError},
    helpers::{Identifier, b64},
};

/// The status of this order.
///
/// Possible values are "pending", "ready", processing", "valid", and "invalid".
#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
    Pending,
    Ready,
    Processing,
    Valid,
    Invalid,
}

/// An order represents a subscribers's request for a certificate from the
/// ACME server, and is used to track the progress of that order through to
/// issuance.
///
/// This must be created through an [`OrderBuilder`].
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    #[serde(skip)]
    pub(crate) account: Option<Arc<Account>>,
    #[serde(skip)]
    pub(crate) url: String,

    /// The status of this order.
    pub status: OrderStatus,
    /// The timestamp after which the server will consider this order
    /// invalid.
    pub expires: Option<String>,
    /// An array of identifier objects that the order pertains to.
    pub identifiers: Vec<Identifier>,
    /// The requested value of the notBefore field in the certificate.
    pub not_before: Option<String>,
    /// The requested value of the notAfter field in the certificate.
    pub not_after: Option<String>,

    /// The error that occurred while processing the order, if any.
    pub error: Option<ServerError>,

    #[serde(rename = "authorizations")]
    /// For pending orders, the authorizations that the client needs to
    /// complete before the requested certificate can be issued. For
    /// final orders (in the "valid" or "invalid" state), the
    /// authorizations that were completed.
    pub(crate) authorization_urls: Vec<String>,
    #[serde(rename = "finalize")]
    /// A URL that a CSR must be POSTed to once all of the order's
    /// authorizations are satisfied to finalize the order.
    pub(crate) finalize_url: String,
    #[serde(rename = "certificate")]
    /// A URL for the certificate that has been issued in response to
    /// this order.
    pub(crate) certificate_url: Option<String>,
}

/// A builder used to create a new [`Order`].
#[derive(Debug)]
pub struct OrderBuilder {
    account: Arc<Account>,

    identifiers: Vec<Identifier>,
    // TODO(lucacasonato): externalAccountBinding
}

impl OrderBuilder {
    pub fn new(account: Arc<Account>) -> Self {
        OrderBuilder {
            account,
            identifiers: vec![],
        }
    }

    /// Add a type `dns` identifier to the list of identifiers for this
    /// order.
    pub fn add_dns_identifier(&mut self, fqdn: String) -> &mut Self {
        self.identifiers.push(Identifier {
            r#type: "dns".to_string(),
            value: fqdn,
        });
        self
    }

    /// This will request a new [`Order`] from the ACME server.
    pub async fn build(&mut self) -> anyhow::Result<Order> {
        let dir = self.account.directory.clone().unwrap();

        let (res, headers) = dir
            .authenticated_request::<_, Order>(
                &dir.new_order_url,
                json!({
                  "identifiers": self.identifiers,
                }),
                self.account.private_key.clone().unwrap(),
                Some(self.account.id.clone()),
            )
            .await?;

        let res: Result<Order, Error> = res.into();
        let mut order = res?;

        let order_url = headers
            .get(reqwest::header::LOCATION)
            .map_or(
                Err(anyhow::anyhow!(
                    "mandatory location header in newOrder response not present"
                )),
                |item| item.to_str().map_err(|e| anyhow::anyhow!(e)),
            )?
            .to_string();

        order.account = Some(self.account.clone());
        order.url = order_url;

        Ok(order)
    }
}

/// A certificate signing request.
pub enum Csr {
    /// Automatic signing takes just a private key. The other details of
    /// the CSR (identifiers, common name, etc), will be automatically
    /// retrieved from the order this is used with.
    Automatic(PKey<Private>),
}

fn gen_csr(pkey: &PKey<Private>, domains: Vec<String>) -> Result<X509Req, Error> {
    if domains.is_empty() {
        return Err(Error::Validation(
            "at least one domain name needs to be supplied",
        ));
    }

    let mut builder = X509Req::builder()?;
    let name = {
        let mut name = X509Name::builder()?;
        name.append_entry_by_text("CN", &domains[0])?;
        name.build()
    };
    builder.set_subject_name(&name)?;

    // Add all domains as SANs
    let san_extension = {
        let mut san = SubjectAlternativeName::new();
        for domain in domains.iter() {
            san.dns(domain);
        }
        san.build(&builder.x509v3_context(None))?
    };
    let mut stack = Stack::new()?;
    stack.push(san_extension)?;
    builder.add_extensions(&stack)?;

    builder.set_pubkey(&pkey)?;
    builder.sign(pkey, MessageDigest::sha256())?;

    Ok(builder.build())
}

impl Order {
    /// Finalize an order (request the final certificate).
    ///
    /// For finalization to complete, the state of the order must be in the
    /// [`OrderStatus::Ready`] state. You can use [`Order::wait_ready`] to wait
    /// until this is the case.
    ///
    /// In most cases this will not complete immediately. You should always
    /// call [`Order::wait_done`] after this operation to wait until the
    /// ACME server has finished finalization, and the certificate is ready
    /// for download.
    pub async fn finalize(&self, csr: Csr) -> anyhow::Result<Order> {
        let csr = match csr {
            Csr::Automatic(pkey) => gen_csr(
                &pkey,
                self.identifiers
                    .iter()
                    .map(|f| f.value.clone())
                    .collect::<Vec<_>>(),
            )?,
        };

        let csr_b64 = b64(&csr.to_der()?);

        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let (res, _) = directory
            .authenticated_request::<_, Order>(
                &self.finalize_url,
                json!({ "csr": csr_b64 }),
                account.private_key.clone().unwrap(),
                Some(account.id.clone()),
            )
            .await?;
        let res: Result<Order, Error> = res.into();
        let mut order = res?;

        order.account = Some(account.clone());
        order.url = self.url.clone();
        Ok(order)
    }

    /// Download the certificate. The order must be in the [`OrderStatus::Valid`]
    /// state for this to complete.
    pub async fn certificate(&self) -> anyhow::Result<String> {
        let certificate_url = match self.certificate_url.clone() {
            Some(certificate_url) => certificate_url,
            None => return Err(anyhow::anyhow!("certificate_url is none")),
        };

        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let bytes = directory
            .authenticated_request_bytes(
                &certificate_url,
                "",
                &account.private_key.clone().unwrap(),
                &Some(account.id.clone()),
            )
            .await?
            .0?;

        // X509::stack_from_pem(&bytes)?

        Ok(String::from_utf8_lossy(&bytes[..]).to_string())
    }

    /// Update the order to match the current server state.
    ///
    /// Most users should use [`Order::wait_ready`] or [`Order::wait_done`].
    pub async fn poll(&self) -> anyhow::Result<Order> {
        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let (res, _) = directory
            .authenticated_request::<_, Order>(
                &self.url,
                json!(""),
                account.private_key.clone().unwrap(),
                Some(account.id.clone()),
            )
            .await?;
        let res: Result<Order, Error> = res.into();
        let mut order = res?;
        order.account = Some(account.clone());
        order.url = self.url.clone();
        Ok(order)
    }

    /// Wait for this order to go into a state other than [`OrderStatus::Pending`].
    ///
    /// This happens when all [`crate::Authorization`]s in this order have been completed
    /// (have the [`crate::AuthorizationStatus::Valid`] state).
    ///
    /// Will complete immediately if the order is already
    /// in one of these states.
    ///
    /// Specify the interval at which to poll the acme server, and how often to
    /// attempt polling before timing out. Polling should not happen faster than
    /// about every 5 seconds to avoid rate limits in the acme server.
    pub async fn wait_ready(
        self,
        poll_interval: Duration,
        attempts: usize,
    ) -> anyhow::Result<Order> {
        let mut order = self;

        let mut i: usize = 0;

        while order.status == OrderStatus::Pending {
            if i >= attempts {
                return Err(anyhow::anyhow!(Error::MaxAttemptsExceeded));
            }
            debug!(target:"certificate", "{:?}, Order still pending. Waiting to poll.", poll_interval);
            tokio::time::sleep(poll_interval).await;
            order = order.poll().await?;
            i += 1;
        }

        Ok(order)
    }

    /// Wait for the order to go into the [`OrderStatus::Valid`]
    /// or [`OrderStatus::Invalid`] state.
    ///
    /// This will happen after the order has gone into the [`OrderStatus::Ready`]
    /// state, and the order has been requested to be finalized.
    ///
    /// Will complete immediately if the order is already
    /// in one of these states.
    ///
    /// Specify the interval at which to poll the acme server, and how often to
    /// attempt polling before timing out. Polling should not happen faster than
    /// about every 5 seconds to avoid rate limits in the acme server.
    pub async fn wait_done(
        self,
        poll_interval: Duration,
        attempts: usize,
    ) -> anyhow::Result<Order> {
        let mut order = self;

        let mut i: usize = 0;

        while order.status == OrderStatus::Pending
            || order.status == OrderStatus::Ready
            || order.status == OrderStatus::Processing
        {
            if i >= attempts {
                return Err(anyhow::anyhow!(Error::MaxAttemptsExceeded));
            }
            debug!(
                target:"certificate", "delay = {:?}, status = {:?} Order not done. Waiting to poll.",
                poll_interval, order.status
            );
            tokio::time::sleep(poll_interval).await;
            order = order.poll().await?;
            i += 1;
        }

        Ok(order)
    }
}
