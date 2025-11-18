use aigw_core::TlsPrivateKey;
use base64::Engine;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use rcgen::{KeyPair, PKCS_ED25519, PKCS_RSA_SHA512};
use serde::{Deserialize, Serialize};

/// This is a identifier for a resource that the ACME server
/// can provision certificates for (a domain).
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    /// The type of identifier.
    pub r#type: String,
    /// The identifier itself.
    pub value: String,
}

pub(crate) fn b64(data: &[u8]) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(data)
}

/// Generate a new Private key
pub fn gen_private_key() -> anyhow::Result<TlsPrivateKey> {
    let key_pair = KeyPair::generate_for(&PKCS_ED25519)?;
    let s = &key_pair.serialize_pem();
    let key = TlsPrivateKey::try_from(s.as_bytes())?;
    Ok(key)
}

pub fn gen_rsa_private_key() -> anyhow::Result<TlsPrivateKey> {
    let key_pair = KeyPair::generate_for(&PKCS_RSA_SHA512)?;
    let s = &key_pair.serialize_pem();
    let key = TlsPrivateKey::try_from(s.as_bytes())?;
    Ok(key)
}
