use aigw_core::{TlsPrivateKey, deserialize_tls_private_key, serialize_tls_private_key};
use base64::Engine;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use bytes::Bytes;
use rcgen::{
    CertificateParams, DistinguishedName, DnType, PKCS_ECDSA_P256_SHA256, SanType,
    string::Ia5String,
};
use ring::{
    rand::SystemRandom,
    signature::{ECDSA_P256_SHA256_FIXED_SIGNING, KeyPair},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;
use sha::{
    sha256,
    utils::{Digest, DigestExt},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tracing::debug;

fn b64(data: &[u8]) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(data)
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct JwsHeader {
    nonce: String,
    alg: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jwk: Option<Jwk>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct Jwk {
    crv: String,
    kty: String,
    x: String,
    y: String,
}

impl Jwk {
    pub fn new(key_pair: &ring::signature::EcdsaKeyPair) -> Jwk {
        let public_key = key_pair.public_key().as_ref();

        // if public_key.len() != 65 || public_key[0] != 4 {
        //     return Err(anyhow::anyhow!("Invalid ECDSA public key"));
        // }

        let x = &public_key[1..33];
        let y = &public_key[33..65];

        Jwk {
            kty: "EC".to_string(),
            x: b64(x),
            y: b64(y),
            crv: "P-256".to_string(),
        }
    }
}

fn jws(
    url: &str,
    nonce: String,
    payload: &str,
    pkey: &TlsPrivateKey,
    account_id: Option<String>,
) -> anyhow::Result<String> {
    let pkcs8 = pkey.0.secret_der();
    let rand = SystemRandom::new();
    let key_pair =
        ring::signature::EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8, &rand)
            .map_err(|_e| anyhow::anyhow!("from_pkcs8 error"))?;

    let payload_b64 = b64(payload.as_bytes());

    let mut header = JwsHeader {
        nonce,
        alg: "ES256".into(),
        url: url.to_string(),
        ..Default::default()
    };

    if let Some(kid) = account_id {
        header.kid = kid.into();
    } else {
        header.jwk = Some(Jwk::new(&key_pair));
    }

    let protected_b64 = b64(&serde_json::to_string(&header)?.into_bytes());

    let signature_b64 = {
        let data = &format!("{}.{}", protected_b64, payload_b64).into_bytes();
        let r = key_pair
            .sign(&rand, data)
            .map_err(|_e| anyhow::anyhow!("sign error"))?;
        b64(r.as_ref())
    };

    Ok(serde_json::to_string(&json!({
      "protected": protected_b64,
      "payload": payload_b64,
      "signature": signature_b64
    }))?)
}

fn extract_nonce_from_response(resp: &reqwest::Response) -> anyhow::Result<Option<String>> {
    let headers = resp.headers();
    let maybe_nonce_res = headers.get("replay-nonce");
    if let Some(hv) = maybe_nonce_res {
        Ok(Some(hv.to_str()?.to_string()))
    } else {
        Ok(None)
    }
}

/// This is an error as returned by the ACME server.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerError {
    /// The type of this error.
    pub r#type: Option<String>,
    /// The human readable title of this error.
    pub title: Option<String>,
    /// The status code of this error.
    pub status: Option<u16>,
    /// The human readable extra description for this error.
    pub detail: Option<String>,
}

/// This is a identifier for a resource that the ACME server
/// can provision certificates for (a domain).
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Identifier {
    /// The type of identifier.
    pub r#type: String,
    /// The identifier itself.
    pub value: String,
}

/// An ACME account. This is used to identify a subscriber to an ACME server.
///
/// This resource should be created through an [`AccountBuilder`].
#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct Account {
    #[serde(skip)]
    pub(crate) directory: Option<Arc<Directory>>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_tls_private_key",
        deserialize_with = "deserialize_tls_private_key"
    )]
    pub(crate) private_key: Option<TlsPrivateKey>,
    pub(crate) key: HashMap<String, String>,
    pub(crate) id: String,
}

/// The status of an [`Account`].
///
/// Possible values are "valid", "deactivated",
/// and "revoked". The value "deactivated" should be used to indicate client-
/// initiated deactivation whereas "revoked" should be used to indicate server-
/// initiated deactivation.
#[derive(Deserialize, Eq, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
enum AccountStatus {
    Valid,
    Deactivated,
    Revoked,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct AccountResponse {
    created_at: String,
    status: AccountStatus,
    key: HashMap<String, String>,
}

/// An builder that is used to create / retrieve an [`Account`] from the
/// ACME server.
#[derive(Debug)]
pub struct AccountBuilder {
    directory: Arc<Directory>,
    contact: Option<Vec<String>>,
    terms_of_service_agreed: Option<bool>,
    only_return_existing: Option<bool>,
    // TODO(lucacasonato): externalAccountBinding
}

impl AccountBuilder {
    /// This creates a new [`AccountBuilder`]. This can be used to create a new
    /// account (if the server has not seen the private key before), or to retrieve
    /// an existing account (using a previously used private key).
    pub fn new(directory: Arc<Directory>) -> Self {
        AccountBuilder {
            directory,
            contact: None,
            terms_of_service_agreed: None,
            only_return_existing: None,
        }
    }

    /// The contact information for the account. For example this could be a
    /// `vec!["email:hello@lcas.dev".to_string()]`. The supported contact types
    /// vary from one ACME server to another.
    pub fn contact(&mut self, contact: Vec<String>) -> &mut Self {
        self.contact = Some(contact);
        self
    }

    /// If you agree to the ACME server terms of service.
    pub fn terms_of_service_agreed(&mut self, terms_of_service_agreed: bool) -> &mut Self {
        self.terms_of_service_agreed = Some(terms_of_service_agreed);
        self
    }

    /// Do not try to create a new account. If this is set, only an existing account
    /// will be returned.
    pub fn only_return_existing(&mut self, only_return_existing: bool) -> &mut Self {
        self.only_return_existing = Some(only_return_existing);
        self
    }

    /// This will create / retrieve an [`Account`] from the ACME server.
    pub async fn build(&mut self) -> anyhow::Result<Arc<Account>> {
        let private_key = Self::gen_private_key()?;

        let url = self.directory.new_account_url.clone();

        let (acc, headers) = self
            .directory
            .authenticated_request::<_, AccountResponse>(
                &url,
                json!({
                  "contact": self.contact,
                  "termsOfServiceAgreed": self.terms_of_service_agreed,
                  "onlyReturnExisting": self.only_return_existing
                }),
                &private_key,
                None,
            )
            .await?;

        debug!(target: "certificate",
            "Account: {}, {:?}, {:?}",
            acc.created_at, acc.key, acc.status
        );

        if acc.status != AccountStatus::Valid {
            return Err(anyhow::anyhow!("Account status error."));
        }

        let account_id = headers
            .get(reqwest::header::LOCATION)
            .ok_or_else(|| anyhow::anyhow!("mandatory location header in newAccount not present"))?
            .to_str()?
            .to_string();

        let account = Account {
            directory: Some(self.directory.clone()),
            private_key: Some(private_key),
            key: acc.key.clone(),
            id: account_id,
        };

        Ok(Arc::new(account))
    }

    /// Generate a new Private key
    fn gen_private_key() -> anyhow::Result<TlsPrivateKey> {
        let key_pair = rcgen::KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
        let s = &key_pair.serialize_pem();
        let key = TlsPrivateKey::try_from(s.as_bytes())?;
        Ok(key)
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
/// The status of this authorization.
///
/// Possible values are "pending", "valid", "invalid", "deactivated",
/// "expired", and "revoked".
pub enum AuthorizationStatus {
    Pending,
    Valid,
    Invalid,
    Deactivated,
    Expired,
    Revoked,
}

/// An autorization represents the server's authorization of a certain
/// domain being represented by an account.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Authorization {
    #[serde(skip)]
    pub(crate) account: Option<Arc<Account>>,
    #[serde(skip)]
    pub(crate) url: String,

    /// The identifier (domain) that the account is authorized to represent.
    pub identifier: Identifier,
    /// The status of this authorization.
    pub status: AuthorizationStatus,
    /// The timestamp after which the server will consider this
    /// authorization invalid.
    pub expires: Option<String>,
    /// For pending authorizations, the challenges that the client can
    /// fulfill in order to prove possession of the identifier. For
    /// valid authorizations, the challenge that was validated. For
    /// invalid authorizations, the challenge that was attempted and
    /// failed.
    pub challenges: Vec<Challenge>,
    /// Whether this authorization was created for a wildcard identifier
    /// (domain).
    pub wildcard: Option<bool>,
}

/// The status of this challenge.
///
/// Possible values are "pending", "processing", "valid", and "invalid".
#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ChallengeStatus {
    Pending,
    Processing,
    Valid,
    Invalid,
}

/// A challenge represents a means for the server to validate
/// that an account has control over an identifier (domain).
///
/// A challenge can only be acquired through an [`Authorization`].
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Challenge {
    #[serde(skip)]
    pub(crate) account: Option<Arc<Account>>,

    /// The type of challenge encoded in the object.
    pub r#type: String,
    /// The URL to which a response can be posted.
    pub(crate) url: String,
    /// The status of this challenge.
    pub status: ChallengeStatus,
    /// The time at which the server validated this challenge.
    pub validated: Option<String>,

    /// Error that occurred while the server was validating the
    /// challenge, if any.
    pub error: Option<ServerError>,

    /// A random value that uniquely identifies the challenge.
    pub token: Option<String>,
}

/// The status of this order.
///
/// Possible values are "pending", "ready", processing", "valid", and "invalid".
#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum OrderStatus {
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
pub(crate) struct Order {
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
pub(crate) struct OrderBuilder {
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

        let (mut order, headers) = dir
            .authenticated_request::<_, Order>(
                &dir.new_order_url,
                json!({
                  "identifiers": self.identifiers,
                }),
                self.account.private_key.as_ref().unwrap(),
                Some(self.account.id.clone()),
            )
            .await?;

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
    pub async fn finalize(&self, pkey: &TlsPrivateKey) -> anyhow::Result<Order> {
        let domains = self
            .identifiers
            .iter()
            .map(|f| f.value.clone())
            .collect::<Vec<_>>();
        let common_name = domains[0].clone();
        let mut san_names = Vec::new();
        for domain in &domains {
            san_names.push(SanType::DnsName(Ia5String::try_from(domain.as_str())?));
        }

        let mut params = CertificateParams::new(domains)?;
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, &common_name);
        params.distinguished_name = distinguished_name;
        params.subject_alt_names = san_names;

        let pkey = rcgen::KeyPair::from_pem(&pkey.try_to_string()?)?;
        let csr = params.serialize_request(&pkey)?;
        let csr = csr.der().as_ref();

        let csr_b64 = b64(csr);

        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let (mut order, _) = directory
            .authenticated_request::<_, Order>(
                &self.finalize_url,
                json!({ "csr": csr_b64 }),
                account.private_key.as_ref().unwrap(),
                Some(account.id.clone()),
            )
            .await?;
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
                account.private_key.as_ref().unwrap(),
                &Some(account.id.clone()),
            )
            .await?
            .0;

        Ok(String::from_utf8_lossy(&bytes[..]).to_string())
    }

    /// Update the order to match the current server state.
    ///
    /// Most users should use [`Order::wait_ready`] or [`Order::wait_done`].
    async fn poll(&self) -> anyhow::Result<Order> {
        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let (mut order, _) = directory
            .authenticated_request::<_, Order>(
                &self.url,
                json!(""),
                account.private_key.as_ref().unwrap(),
                Some(account.id.clone()),
            )
            .await?;
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
    pub(crate) async fn wait_ready(
        self,
        poll_interval: Duration,
        attempts: usize,
    ) -> anyhow::Result<Order> {
        let mut order = self;

        let mut i: usize = 0;

        while order.status == OrderStatus::Pending {
            if i >= attempts {
                return Err(anyhow::anyhow!(
                    "the maximum poll attempts have been exceeded"
                ));
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
    pub(crate) async fn wait_done(
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
                return Err(anyhow::anyhow!(
                    "the maximum poll attempts have been exceeded"
                ));
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

impl Order {
    /// Retrieve all of the [`Authorization`]s needed for this order.
    ///
    /// The authorization may already be in a `Valid` state, if an
    /// authorization for this identifier was already completed through
    /// a seperate order.
    pub async fn authorizations(&self) -> anyhow::Result<Vec<Authorization>> {
        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let mut authorizations = vec![];

        for authorization_url in self.authorization_urls.clone() {
            let (mut authorization, _) = directory
                .authenticated_request::<_, Authorization>(
                    &authorization_url,
                    "",
                    account.private_key.as_ref().unwrap(),
                    Some(account.id.clone()),
                )
                .await?;
            authorization.account = Some(account.clone());
            authorization.url = authorization_url;
            for challenge in &mut authorization.challenges {
                challenge.account = Some(account.clone())
            }
            authorizations.push(authorization)
        }

        Ok(authorizations)
    }
}

impl Authorization {
    /// Get a certain type of challenge to complete.
    ///
    /// Example: `http-01`, or `dns-01`
    pub fn get_challenge(&self, r#type: &str) -> Option<Challenge> {
        for challenge in &self.challenges {
            if challenge.r#type == r#type {
                return Some(challenge.clone());
            }
        }
        None
    }

    /// Update the authorization to match the current server state.
    ///
    /// Most users should use [`Authorization::wait_done`].
    pub async fn poll(self) -> anyhow::Result<Authorization> {
        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let (mut authorization, _) = directory
            .authenticated_request::<_, Authorization>(
                &self.url,
                json!(""),
                account.private_key.as_ref().unwrap(),
                Some(account.id.clone()),
            )
            .await?;
        authorization.url = self.url.clone();
        authorization.account = Some(account.clone());
        Ok(authorization)
    }

    /// Wait for the authorization to go into a state other than
    /// [`AuthorizationStatus::Pending`].
    ///
    /// This will only happen once one of the challenges in an authorization
    /// is completed. You can use [`Challenge::wait_done`] to wait until
    /// this is the case.
    ///
    /// Will complete immediately if the authorization is already in a
    /// state other than [`AuthorizationStatus::Pending`].
    ///
    /// Specify the interval at which to poll the acme server, and how often to
    /// attempt polling before timing out. Polling should not happen faster than
    /// about every 5 seconds to avoid rate limits in the acme server.
    pub async fn wait_done(
        self,
        poll_interval: Duration,
        attempts: usize,
    ) -> anyhow::Result<Authorization> {
        let mut authorization = self;

        let mut i: usize = 0;

        while authorization.status == AuthorizationStatus::Pending {
            if i >= attempts {
                return Err(anyhow::anyhow!(
                    "the maximum poll attempts have been exceeded"
                ));
            }
            debug!(target:"certificate",
                "{:?},Authorization still pending. Waiting to poll.",
                poll_interval
            );
            tokio::time::sleep(poll_interval).await;
            authorization = authorization.poll().await?;
            i += 1;
        }

        Ok(authorization)
    }
}

impl Challenge {
    /// The key authorization is the token that the HTTP01 challenge
    /// should be serving for the ACME server to inspect.
    pub fn key_authorization(&self) -> anyhow::Result<Option<String>> {
        if let Some(token) = self.token.clone() {
            let account = self.account.clone().unwrap();

            let pkey = account.private_key.as_ref().unwrap();
            let pkcs8 = pkey.0.secret_der();

            let rand = SystemRandom::new();
            let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
                &ECDSA_P256_SHA256_FIXED_SIGNING,
                pkcs8,
                &rand,
            )
            .map_err(|_e| anyhow::anyhow!("from_pkcs8 error"))?;

            let data = &serde_json::to_string(&Jwk::new(&key_pair))?.into_bytes();
            let mut sha = sha256::Sha256::default();
            sha.digest(data);

            let key_authorization = format!("{}.{}", token, b64(&sha.to_bytes()));

            Ok(Some(key_authorization))
        } else {
            Ok(None)
        }
    }

    /// Initiate validation of the challenge by the ACME server.
    ///
    /// Before calling this method, you should have set up your challenge token
    /// so it is available for the ACME server to check.
    ///
    /// In most cases this will not complete immediately. You should always
    /// call [`Challenge::wait_done`] after this operation to wait until the
    /// ACME server has finished validation.
    pub async fn validate(&self) -> anyhow::Result<Challenge> {
        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let (mut challenge, _) = directory
            .authenticated_request::<_, Challenge>(
                &self.url,
                json!({}),
                account.private_key.as_ref().unwrap(),
                Some(account.id.clone()),
            )
            .await?;
        challenge.account = Some(account.clone());

        Ok(challenge)
    }

    /// Update the challenge to match the current server state.
    ///
    /// Most users should use [`Challenge::wait_done`].
    pub async fn poll(&self) -> anyhow::Result<Challenge> {
        let account = self.account.clone().unwrap();
        let directory = account.directory.clone().unwrap();

        let (mut challenge, _) = directory
            .authenticated_request::<_, Challenge>(
                &self.url,
                json!(""),
                account.private_key.as_ref().unwrap(),
                Some(account.id.clone()),
            )
            .await?;
        challenge.account = Some(account.clone());
        Ok(challenge)
    }

    /// Wait for the authorization to go into the [`AuthorizationStatus::Valid`]
    /// or [`AuthorizationStatus::Invalid`] state.
    ///
    /// Will complete immediately if the authorization is already
    /// in one of these states.
    ///
    /// Specify the interval at which to poll the acme server, and how often to
    /// attempt polling before timing out. Polling should not happen faster than
    /// about every 5 seconds to avoid rate limits in the acme server.
    pub async fn wait_done(
        self,
        poll_interval: Duration,
        attempts: usize,
    ) -> anyhow::Result<Challenge> {
        let mut challenge = self;

        let mut i: usize = 0;

        while challenge.status == ChallengeStatus::Pending
            || challenge.status == ChallengeStatus::Processing
        {
            if i >= attempts {
                return Err(anyhow::anyhow!(
                    "the maximum poll attempts have been exceeded"
                ));
            }
            debug!(target:"certificate",
                "{:?}, {:?}, Challenge not done. Waiting to poll.",
                poll_interval, challenge.status
            );
            tokio::time::sleep(poll_interval).await;
            challenge = challenge.poll().await?;
            i += 1;
        }

        Ok(challenge)
    }
}

/// An builder that is used create a [`Directory`].
pub struct DirectoryBuilder {
    url: String,
    http_client: Option<reqwest::Client>,
}

impl DirectoryBuilder {
    /// Creates a new builder with the specified directory root URL.
    ///
    /// Let's Encrypt: `https://acme-v02.api.letsencrypt.org/directory`
    ///
    /// Let's Encrypt Staging: `https://acme-staging-v02.api.letsencrypt.org/directory`
    pub fn new(url: String) -> Self {
        DirectoryBuilder {
            url,
            http_client: None,
        }
    }

    /// Build a [`Directory`] using the given parameters.
    ///
    /// If no http client is specified, a default client will be created using
    /// the webpki trust roots.
    pub async fn build(&mut self) -> anyhow::Result<Arc<Directory>> {
        let http_client = self.http_client.clone().unwrap_or_default();

        let resp = http_client.get(&self.url).send().await?;
        let mut dir = resp.json::<Directory>().await?;

        dir.http_client = http_client;
        dir.nonce = Mutex::new(None);

        Ok(Arc::new(dir))
    }
}

/// A directory is the resource representing how to reach an ACME server.
///
/// Must be created through a [`DirectoryBuilder`].
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    #[serde(skip)]
    pub(crate) http_client: reqwest::Client,
    #[serde(skip)]
    pub(crate) nonce: Mutex<Option<String>>,
    #[serde(rename = "newNonce")]
    pub(crate) new_nonce_url: String,
    #[serde(rename = "newAccount")]
    pub(crate) new_account_url: String,
    #[serde(rename = "newOrder")]
    pub(crate) new_order_url: String,
    #[serde(rename = "revokeCert")]
    pub(crate) revoke_cert_url: String,
    #[serde(rename = "keyChange")]
    pub(crate) key_change_url: String,
}

impl Directory {
    async fn get_nonce(&self) -> anyhow::Result<String> {
        let maybe_nonce = {
            let mut guard = self.nonce.lock().unwrap();
            (*guard).take()
        };

        if let Some(nonce) = maybe_nonce {
            return Ok(nonce);
        }
        let resp = self.http_client.get(&self.new_nonce_url).send().await?;
        let maybe_nonce = extract_nonce_from_response(&resp)?;
        match maybe_nonce {
            Some(nonce) => Ok(nonce),
            None => Err(anyhow::anyhow!("newNonce request must return a nonce")),
        }
    }

    async fn authenticated_request_raw(
        &self,
        url: &str,
        payload: &str,
        pkey: &TlsPrivateKey,
        account_id: &Option<String>,
    ) -> anyhow::Result<reqwest::Response> {
        let nonce = self.get_nonce().await?;
        let body = jws(url, nonce, payload, pkey, account_id.clone())?;
        let resp = self
            .http_client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/jose+json")
            .body(body)
            .send()
            .await?;

        if let Some(nonce) = extract_nonce_from_response(&resp)? {
            let mut guard = self.nonce.lock().unwrap();
            *guard = Some(nonce);
        }

        Ok(resp)
    }

    async fn authenticated_request_bytes(
        &self,
        url: &str,
        payload: &str,
        pkey: &TlsPrivateKey,
        account_id: &Option<String>,
    ) -> anyhow::Result<(Bytes, reqwest::header::HeaderMap)> {
        let mut attempt = 0;

        loop {
            attempt += 1;

            let resp = self
                .authenticated_request_raw(url, payload, pkey, account_id)
                .await?;

            let headers = resp.headers().clone();
            if resp.status().is_success() {
                return Ok((resp.bytes().await?, headers));
            }

            let err: ServerError = resp.json().await?;

            if let Some(typ) = err.r#type.clone()
                && &typ == "urn:ietf:params:acme:error:badNonce"
                && attempt <= 3
            {
                debug!(target:"certificate", "{} bad nonce, retrying", attempt);
                continue;
            }

            return Err(anyhow::anyhow!(serde_json::to_string(&err)?));
        }
    }

    async fn authenticated_request<T, R>(
        &self,
        url: &str,
        payload: T,
        pkey: &TlsPrivateKey,
        account_id: Option<String>,
    ) -> anyhow::Result<(R, reqwest::header::HeaderMap)>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let payload = serde_json::to_string(&payload)?;
        let payload = if payload == "\"\"" {
            "".to_string()
        } else {
            payload
        };

        let (bytes, headers) = self
            .authenticated_request_bytes(url, &payload, pkey, &account_id)
            .await?;

        let val: R = serde_json::from_slice(&bytes)?;

        Ok((val, headers))
    }
}
