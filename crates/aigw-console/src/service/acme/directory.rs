use std::sync::{Arc, Mutex};

use boring::pkey::{PKey, Private};
use bytes::Bytes;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tracing::debug;

use crate::service::acme::{
    error::{Error, ServerError, ServerResult},
    jws::jws,
};

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
    pub async fn build(&mut self) -> Result<Arc<Directory>, Error> {
        let http_client = self
            .http_client
            .clone()
            .unwrap_or_else(reqwest::Client::new);

        let resp = http_client.get(&self.url).send().await?;

        let res: Result<Directory, Error> = resp.json::<ServerResult<Directory>>().await?.into();
        let mut dir = res?;

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

fn extract_nonce_from_response(resp: &reqwest::Response) -> anyhow::Result<Option<String>> {
    let headers = resp.headers();
    let maybe_nonce_res = headers.get("replay-nonce");
    if let Some(hv) = maybe_nonce_res {
        Ok(Some(hv.to_str()?.to_string()))
    } else {
        Ok(None)
    }
}

impl Directory {
    pub(crate) async fn get_nonce(&self) -> anyhow::Result<String> {
        let maybe_nonce = {
            let mut guard = self.nonce.lock().unwrap();
            std::mem::replace(&mut *guard, None)
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
        pkey: &PKey<Private>,
        account_id: &Option<String>,
    ) -> anyhow::Result<reqwest::Response> {
        let nonce = self.get_nonce().await?;
        let body = jws(url, nonce, &payload, pkey, account_id.clone())?;
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

    pub(crate) async fn authenticated_request_bytes(
        &self,
        url: &str,
        payload: &str,
        pkey: &PKey<Private>,
        account_id: &Option<String>,
    ) -> anyhow::Result<(Result<Bytes, ServerError>, reqwest::header::HeaderMap)> {
        let mut attempt = 0;

        loop {
            attempt += 1;

            let resp = self
                .authenticated_request_raw(url, &payload, &pkey, &account_id)
                .await?;

            let headers = resp.headers().clone();

            if resp.status().is_success() {
                return Ok((Ok(resp.bytes().await?), headers));
            }

            let err: ServerError = resp.json().await?;

            if let Some(typ) = err.r#type.clone() {
                if &typ == "urn:ietf:params:acme:error:badNonce" && attempt <= 3 {
                    debug!(target:"certificate", "{} bad nonce, retrying", attempt);
                    continue;
                }
            }

            return Ok((Err(err), headers));
        }
    }

    pub(crate) async fn authenticated_request<T, R>(
        &self,
        url: &str,
        payload: T,
        pkey: PKey<Private>,
        account_id: Option<String>,
    ) -> anyhow::Result<(ServerResult<R>, reqwest::header::HeaderMap)>
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

        let (res, headers) = self
            .authenticated_request_bytes(url, &payload, &pkey, &account_id)
            .await?;
        let bytes = match res {
            Ok(bytes) => bytes,
            Err(err) => return Ok((ServerResult::Err(err), headers)),
        };

        let val: R = serde_json::from_slice(&bytes)?;

        Ok((ServerResult::Ok(val), headers))
    }
}
