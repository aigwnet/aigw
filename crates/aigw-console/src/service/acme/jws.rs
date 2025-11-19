use aigw_core::TlsPrivateKey;
use anyhow::Ok;
use ring::{
    rand::SystemRandom,
    signature::{ECDSA_P256_SHA256_FIXED_SIGNING, KeyPair},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::service::b64;

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
pub(crate) struct Jwk {
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

pub(crate) fn jws(
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
