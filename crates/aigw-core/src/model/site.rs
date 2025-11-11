use std::{path::PathBuf, sync::Arc};

use base64::{Engine, prelude::BASE64_STANDARD};
use pingora_core::tls::{
    pkey::{PKey, Private},
    x509::X509,
};
use serde::{Deserialize, Serialize};

use super::location::ProxyLocation;

#[derive(Serialize, Deserialize)]
pub struct Site {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    // cluster name
    pub cluster: String,
    // "example.com"
    pub name: String,
    // ["www.example.com", "abc.example.com"]
    #[serde(default)]
    pub alt_names: Vec<String>,

    pub auto_index: bool,
    // root dir
    pub root_dir: Option<PathBuf>,

    #[serde(default)]
    pub tls_on: bool,

    #[serde(default)]
    pub tls_enforce: bool,

    #[serde(default)]
    pub acme_on: bool,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_cert",
        deserialize_with = "deserialize_cert"
    )]
    pub tls_cert: Option<DynamicCert>,

    pub tls_cert_start_date: Option<String>,

    pub tls_cert_end_date: Option<String>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_tls_private_key",
        deserialize_with = "deserialize_tls_private_key"
    )]
    pub tls_private_key: Option<TlsPrivateKey>,
    // For example, `/`, `^~ /api/apps/latest/`
    #[serde(
        default,
        serialize_with = "serialize_locs",
        deserialize_with = "deserialize_locs"
    )]
    pub locations: Vec<Arc<ProxyLocation>>,

    pub rate_limit: isize,
    pub rate_limit_unit: u64,
}

fn serialize_tls_private_key<S>(
    value: &Option<TlsPrivateKey>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(value) => {
            let key = value
                .try_to_string()
                .map_err(|_| serde::ser::Error::custom("Private key pem format error"))?;
            serializer.serialize_str(&BASE64_STANDARD.encode(&key))
        }
        None => serializer.serialize_str(""),
    }
}

fn deserialize_tls_private_key<'de, D>(deserializer: D) -> Result<Option<TlsPrivateKey>, D::Error>
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
    Ok(Some(TlsPrivateKey(key)))
}

fn serialize_locs<S>(value: &[Arc<ProxyLocation>], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    value
        .iter()
        .map(|arc| arc.as_ref())
        .collect::<Vec<_>>()
        .serialize(serializer)
}

fn deserialize_locs<'de, D>(deserializer: D) -> Result<Vec<Arc<ProxyLocation>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let items = Vec::<ProxyLocation>::deserialize(deserializer)?
        .into_iter()
        .map(Arc::new)
        .collect();
    Ok(items)
}

#[derive(Debug, Clone)]
pub struct TlsPrivateKey(PKey<Private>);

impl TlsPrivateKey {
    pub fn try_to_string(&self) -> anyhow::Result<String> {
        let key = self.0.private_key_to_pem_pkcs8()?;
        Ok(unsafe { String::from_utf8_unchecked(key) })
    }
}

impl TryFrom<&[u8]> for TlsPrivateKey {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let key = PKey::private_key_from_pem(value)?;
        Ok(TlsPrivateKey(key))
    }
}

impl AsRef<PKey<Private>> for TlsPrivateKey {
    fn as_ref(&self) -> &PKey<Private> {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct DynamicCert {
    pub cert: X509,
    pub cert_chain: Vec<X509>,
}

impl DynamicCert {
    pub fn try_to_string(&self) -> anyhow::Result<String> {
        let mut s = String::new();
        let cert = self.cert.to_pem()?;
        s += unsafe { String::from_utf8_unchecked(cert) }.as_str();

        for cert in &self.cert_chain {
            let cert = cert.to_pem()?;
            s += unsafe { String::from_utf8_unchecked(cert) }.as_str();
        }

        Ok(s)
    }
}

impl TryFrom<&[u8]> for DynamicCert {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let pems = pem::parse_many(value)?;

        let mut certs = vec![];
        for p in pems {
            let cert = X509::from_pem(p.to_string().as_bytes())?;
            certs.push(cert);
        }

        if certs.is_empty() {
            return Err(anyhow::anyhow!("Cert is empty"));
        }

        let cert = DynamicCert {
            cert: certs.remove(0),
            cert_chain: certs,
        };

        Ok(cert)
    }
}

fn serialize_cert<S>(value: &Option<DynamicCert>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(value) => {
            let s = value
                .try_to_string()
                .map_err(|_| serde::ser::Error::custom("Cert pem format error"))?;
            let s = s.trim();
            if s.is_empty() {
                serializer.serialize_str("")
            } else {
                serializer.serialize_str(&BASE64_STANDARD.encode(&s))
            }
        }
        None => serializer.serialize_str(""),
    }
}

fn deserialize_cert<'de, D>(deserializer: D) -> Result<Option<DynamicCert>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let cert = String::deserialize(deserializer)?;
    if cert.trim().is_empty() {
        return Ok(None);
    }
    let cert = BASE64_STANDARD
        .decode(cert)
        .map_err(|_| serde::de::Error::custom("Cert base64 decode error"))?;

    let cert = DynamicCert::try_from(&cert[..])
        .map_err(|_| serde::de::Error::custom("Cert pem format error"))?;

    Ok(Some(cert))
}
