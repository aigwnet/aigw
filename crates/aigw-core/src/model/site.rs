use std::{path::PathBuf, sync::Arc};

use base64::{Engine, prelude::BASE64_STANDARD};
use pem::Pem;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use x509_parser::prelude::{FromDer, X509Certificate};

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

    #[serde(skip)]
    pub certified_key: Option<Arc<rustls::sign::CertifiedKey>>,
}

pub fn serialize_tls_private_key<S>(
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

pub fn deserialize_tls_private_key<'de, D>(
    deserializer: D,
) -> Result<Option<TlsPrivateKey>, D::Error>
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

    let key = TlsPrivateKey::try_from(&key[..])
        .map_err(|_| serde::de::Error::custom("Private key pem format error"))?;
    Ok(Some(key))
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

#[derive(Debug)]
pub struct TlsPrivateKey(pub PrivateKeyDer<'static>);

impl TlsPrivateKey {
    pub fn try_to_string(&self) -> anyhow::Result<String> {
        let key = self.0.secret_der();
        let pem = Pem::new("PRIVATE KEY", key);
        Ok(pem.to_string())
    }
}

impl TryFrom<&[u8]> for TlsPrivateKey {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let pem = pem::parse(value)?;
        let key = PrivateKeyDer::try_from(pem.contents().to_vec())
            .map_err(|_| anyhow::anyhow!("Parse TlsPrivateKey error."))?;
        Ok(TlsPrivateKey(key))
    }
}

#[derive(Debug, Clone)]
pub struct DynamicCert {
    pub cert: X509Cert,
    pub cert_chain: Vec<X509Cert>,
}

#[derive(Debug, Clone)]
pub struct X509Cert {
    subject: String,
    not_after: OffsetDateTime,
    not_before: OffsetDateTime,
    cert: CertificateDer<'static>,
}
impl X509Cert {
    pub fn from_pem(pem: &[u8]) -> anyhow::Result<Self> {
        let pem = pem::parse(pem)?;
        if pem.tag() != "CERTIFICATE" {
            return Err(anyhow::anyhow!("PEM is not a certificate"));
        }
        let (_, c) = X509Certificate::from_der(pem.contents())?;
        Ok(Self {
            subject: c.subject().to_string(),
            not_after: c.validity().not_after.to_datetime(),
            not_before: c.validity().not_before.to_datetime(),
            cert: CertificateDer::from(pem.contents().to_vec()),
        })
    }

    pub fn to_pem(&self) -> String {
        let data = pem::Pem::new("CERTIFICATE", self.cert.as_ref());
        data.to_string()
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn not_after(&self) -> OffsetDateTime {
        self.not_after
    }

    pub fn not_before(&self) -> OffsetDateTime {
        self.not_before
    }

    pub fn cert(&self) -> &CertificateDer<'static> {
        &self.cert
    }
}

impl DynamicCert {
    pub fn try_to_string(&self) -> anyhow::Result<String> {
        let mut s = String::new();
        let cert = self.cert.to_pem();
        s += cert.as_str();

        for cert in &self.cert_chain {
            let cert = cert.to_pem();
            s += cert.as_str();
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
            let cert = X509Cert::from_pem(p.to_string().as_bytes())?;
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
                serializer.serialize_str(&BASE64_STANDARD.encode(s))
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
