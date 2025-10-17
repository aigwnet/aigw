use std::num::NonZeroU32;

use ring::{
    digest, pbkdf2,
    signature::{self, ED25519_PUBLIC_KEY_LEN, Ed25519KeyPair, KeyPair},
};
const SALT: &[u8; 32] = b"flamingoflamiNGOFlamingoflamingo";

pub struct Signature {
    key_pair: Ed25519KeyPair,
}

impl Signature {
    pub fn new(password: &str) -> Self {
        let key_pair = Signature::keypair_from_password(password);
        Self { key_pair }
    }

    fn keypair_from_password(password: &str) -> Ed25519KeyPair {
        let mut key = [0; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(4096).unwrap(),
            SALT,
            password.as_bytes(),
            &mut key,
        );
        Ed25519KeyPair::from_seed_unchecked(&key).unwrap()
    }

    ///
    /// 签名
    ///
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        self.key_pair.sign(data).as_ref().to_vec()
    }

    ///
    /// 验签
    ///
    pub fn verify(
        &self,
        public_key_salt: &[u8],
        public_key_hash: &[u8],
        sign: &[u8],
        data: &[u8],
    ) -> anyhow::Result<bool> {
        let hash = calculate_hash(
            self.key_pair.public_key().as_ref(),
            public_key_salt.try_into()?,
        );
        let hash_init: [u8; 4] = public_key_hash.try_into()?;
        if hash != hash_init {
            return Err(anyhow::anyhow!("invalid key"));
        }

        let public_key = signature::UnparsedPublicKey::new(
            &ring::signature::ED25519,
            self.key_pair.public_key(),
        );
        if public_key.verify(data, sign).is_err() {
            Err(anyhow::anyhow!("invalid signature"))
        } else {
            Ok(true)
        }
    }

    pub fn get_public_key(&self) -> &[u8] {
        self.key_pair.public_key().as_ref()
    }
}

pub(crate) fn calculate_hash(key: &[u8], salt: &[u8; 4]) -> [u8; 4] {
    let mut data = [0; ED25519_PUBLIC_KEY_LEN + 4];
    data[..ED25519_PUBLIC_KEY_LEN].clone_from_slice(key);
    data[ED25519_PUBLIC_KEY_LEN..].clone_from_slice(salt);
    let hash = digest::digest(&digest::SHA256, &data);
    let mut short_hash = [0; 4];
    short_hash.clone_from_slice(&hash.as_ref()[..4]);
    short_hash
}
