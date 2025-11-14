use crate::ssl::{
    pkey::{PKey, Private},
    rsa::Rsa,
};
use base64::Engine;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

use crate::service::acme::error::Error;

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

/// Generate a new RSA private key using the specified size,
/// using the system random.
pub fn gen_rsa_private_key(bits: u32) -> Result<PKey<Private>, Error> {
    let rsa = Rsa::generate(bits)?;
    let key = PKey::from_rsa(rsa)?;
    Ok(key)
}
