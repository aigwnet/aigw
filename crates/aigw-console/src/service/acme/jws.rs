use crate::service::acme::helpers::b64;
use aigw_core::TlsPrivateKey;
use ring::{
    rand::SystemRandom,
    signature::{RSA_PKCS1_SHA256, RsaKeyPair},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
    e: String,
    kty: String,
    n: String,
}

impl Jwk {
    pub fn new(pkey: &TlsPrivateKey) -> Jwk {
        Jwk {
            e: b64(&pkey.0.rsa().unwrap().e().to_vec()),
            kty: "RSA".to_string(),
            n: b64(&pkey.0.rsa().unwrap().n().to_vec()),
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
    let payload_b64 = b64(payload.as_bytes());

    let mut header = JwsHeader {
        nonce,
        alg: "RS256".into(),
        url: url.to_string(),
        ..Default::default()
    };

    if let Some(kid) = account_id {
        header.kid = kid.into();
    } else {
        header.jwk = Some(Jwk::new(pkey));
    }

    let protected_b64 = b64(&serde_json::to_string(&header)?.into_bytes());

    let signature_b64 = {
        let data = &format!("{}.{}", protected_b64, payload_b64).into_bytes();
        // PKCS#8
        let key_pair = RsaKeyPair::from_pkcs8(&pkey.0.private_key_to_pkcs8()?)
            .map_err(|_e| anyhow::anyhow!("from_der error"))?;

        let rng = SystemRandom::new();
        let mut r = vec![0; key_pair.public().modulus_len()];
        let _ = key_pair
            .sign(&RSA_PKCS1_SHA256, &rng, data, &mut r)
            .map_err(|_e| anyhow::anyhow!("sign error"))?;
        b64(&r)
    };

    Ok(serde_json::to_string(&json!({
      "protected": protected_b64,
      "payload": payload_b64,
      "signature": signature_b64
    }))?)
}

// #[cfg(test)]
// mod tests {
//     use openssl::{hash::MessageDigest, sign::Signer};
//     use rcgen::{KeyPair, SigningKey};
//     use ring::{
//         rand::SystemRandom,
//         signature::{RSA_PKCS1_SHA256, RsaKeyPair},
//     };

//     use crate::service::acme::helpers::{b64, gen_rsa_private_key};

//     #[test]
//     fn test() -> anyhow::Result<()> {
//         let pkey = gen_rsa_private_key()?;

//         println!("{}", &pkey.try_to_string()?);
//         let mut signer = Signer::new(MessageDigest::sha256(), &pkey.0)?;

//         let ssss = &format!("{}.{}", "1111", "22222").into_bytes();
//         signer.update(ssss)?;
//         let s = b64(&signer.sign_to_vec()?);

//         println!("{}", s);

//         let key_pair = RsaKeyPair::from_pkcs8(&pkey.0.private_key_to_pkcs8()?).map_err(|e| {
//             println!("{:?}", e);
//             anyhow::anyhow!("from_der error")
//         })?;

//         let rng = SystemRandom::new();
//         let mut r = vec![0; key_pair.public().modulus_len()];
//         let _ = key_pair
//             .sign(&RSA_PKCS1_SHA256, &rng, ssss, &mut r)
//             .map_err(|e| {
//                 println!("{:?}", e);
//                 anyhow::anyhow!("sign error")
//             })?;

//         let s = b64(&r);

//         println!("{}", s);

//         let pkey = KeyPair::from_pem(&pkey.try_to_string()?)?;
//         let r = pkey.sign(ssss)?;
//         let s = b64(&r);
//         println!("{}", s);

//         Ok(())
//     }
// }
