use aigw_core::{TlsPrivateKey, deserialize_tls_private_key, serialize_tls_private_key};
use rcgen::{KeyPair, PKCS_ECDSA_P256_SHA256};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tracing::debug;

use crate::service::acme::{directory::Directory, error::Error};

/// An ACME account. This is used to identify a subscriber to an ACME server.
///
/// This resource should be created through an [`AccountBuilder`].
#[derive(Deserialize, Serialize, Debug)]
pub struct Account {
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
pub enum AccountStatus {
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

        let (res, headers) = self
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
        let res: Result<AccountResponse, Error> = res.into();

        let acc = res?;

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
        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
        let s = &key_pair.serialize_pem();
        let key = TlsPrivateKey::try_from(s.as_bytes())?;
        Ok(key)
    }
}
