use std::{collections::HashMap, sync::Arc};

use base64::{Engine, prelude::BASE64_STANDARD};
use boring::pkey::{PKey, Private};
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::service::acme::{directory::Directory, error::Error, helpers::gen_rsa_private_key};

/// An ACME account. This is used to identify a subscriber to an ACME server.
///
/// This resource should be created through an [`AccountBuilder`].
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Account {
    #[serde(skip)]
    pub(crate) directory: Option<Arc<Directory>>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_private_key",
        deserialize_with = "deserialize_private_key"
    )]
    pub(crate) private_key: Option<PKey<Private>>,
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

fn serialize_private_key<S>(value: &Option<PKey<Private>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(value) => {
            let key = value
                .private_key_to_pem_pkcs8()
                .map_err(|_| serde::ser::Error::custom("Private key pem format error"))?;
            let key = unsafe { String::from_utf8_unchecked(key) };
            serializer.serialize_str(&BASE64_STANDARD.encode(&key))
        }
        None => serializer.serialize_str(""),
    }
}

fn deserialize_private_key<'de, D>(deserializer: D) -> Result<Option<PKey<Private>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let key = String::deserialize(deserializer)?;
    if key.trim().is_empty() {
        return Ok(None);
    }
    let key = BASE64_STANDARD
        .decode(key)
        .map_err(|_| serde::de::Error::custom("Private key base64 decode error"))?;
    let key = PKey::private_key_from_pem(&key)
        .map_err(|_| serde::de::Error::custom("Private key pem format error"))?;
    Ok(Some(key))
}

/// An builder that is used to create / retrieve an [`Account`] from the
/// ACME server.
#[derive(Debug)]
pub struct AccountBuilder {
    directory: Arc<Directory>,

    private_key: Option<PKey<Private>>,

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
            private_key: None,
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
    ///
    /// If the [`AccountBuilder`] does not contain a private key, a new
    /// 4096 bit RSA key will be generated (using the system random). If
    /// a key is generated, it can be retrieved from the created [`Account`]
    /// through the [`Account::private_key`] method.
    pub async fn build(&mut self) -> anyhow::Result<Arc<Account>> {
        let private_key = if let Some(private_key) = self.private_key.clone() {
            private_key
        } else {
            gen_rsa_private_key(4096)?
        };

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
                private_key.clone(),
                None,
            )
            .await?;
        let res: Result<AccountResponse, Error> = res.into();

        let acc = res?;

        debug!(
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
}

#[cfg(test)]
mod tests {

    use crate::service::DirectoryBuilder;

    use super::*;

    #[tokio::test]
    async fn test_account() -> anyhow::Result<()> {
        let dir =
            DirectoryBuilder::new("https://acme-v02.api.letsencrypt.org/directory".to_string())
                .build()
                .await?;

        let contact = "mailto:lijunbox@126.com".to_string();

        // Create an ACME account to use for the order. For production
        // purposes, you should keep the account (and private key), so
        // you can renew your certificate easily.
        let mut builder = AccountBuilder::new(dir.clone());
        builder.contact(vec![contact]);
        builder.terms_of_service_agreed(true);
        builder.only_return_existing(false);
        let account = builder.build().await;

        if let Err(err) = account {
            println!("err: {:?}", err);
        }

        Ok(())
    }
}
