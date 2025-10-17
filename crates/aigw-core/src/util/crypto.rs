use std::io::{Cursor, Read, Write};

use byteorder::{ReadBytesExt, WriteBytesExt};
use ring::{
    aead::{self, LessSafeKey, NONCE_LEN, UnboundKey},
    agreement::{EphemeralPrivateKey, UnparsedPublicKey, X25519, agree_ephemeral},
    rand::{SecureRandom, SystemRandom},
};

use crate::protocol::Algorithm;

use super::buf::Buffer;

pub struct CryptoCore {
    key: LessSafeKey,
    rand: SystemRandom,
}

const TAG_LEN: usize = 16;
const EXTRA_LEN: usize = 8;

impl CryptoCore {
    pub fn new(private_key: EphemeralPrivateKey, algorithm: &Algorithm, public_key: &[u8]) -> Self {
        let ecdh_public_key = UnparsedPublicKey::new(&X25519, public_key.into());
        let master_key =
            CryptoCore::derive_master_key(algorithm.into(), private_key, &ecdh_public_key);
        Self {
            key: master_key,
            rand: SystemRandom::new(),
        }
    }

    pub fn encrypt(&self, buf: &mut Buffer) {
        let data_start = buf.get_start();
        let data_length = buf.len();
        assert!(buf.get_start() >= EXTRA_LEN);

        // insert 8 bytes before real data
        // insert 16 bytes after real data
        buf.set_start(data_start - EXTRA_LEN);
        buf.set_len(data_length + EXTRA_LEN + TAG_LEN);

        let (extra, data_and_tag) = buf.message_mut().split_at_mut(EXTRA_LEN);
        let (data, tag_space) = data_and_tag.split_at_mut(data_length);

        let mut nonce = [0; NONCE_LEN];
        {
            self.rand
                .fill(&mut nonce[5..])
                .expect("Failed to obtain random bytes");

            let mut extra = Cursor::new(extra);
            extra.write_u8(0).unwrap();
            extra.write_all(&nonce[5..]).unwrap();
        }

        let nonce = aead::Nonce::assume_unique_for_key(nonce);

        let tag = self
            .key
            .seal_in_place_separate_tag(nonce, aead::Aad::empty(), data)
            .expect("Failed to encrypt");
        tag_space.clone_from_slice(tag.as_ref());
    }

    ///
    /// Decrypt message
    ///
    pub fn decrypt(&self, buf: &mut Buffer) -> anyhow::Result<()> {
        assert!(buf.len() >= EXTRA_LEN + TAG_LEN);
        let (extra, data_and_tag) = buf.message_mut().split_at_mut(EXTRA_LEN);
        let mut nonce = [0; NONCE_LEN];
        {
            let mut extra = Cursor::new(extra);
            extra.read_u8()?;
            extra.read_exact(&mut nonce[5..])?;
        }

        // decrypt
        let crypto_nonce = aead::Nonce::assume_unique_for_key(nonce);
        self.key
            .open_in_place(crypto_nonce, aead::Aad::empty(), data_and_tag)
            .map_err(|e| anyhow::anyhow!(e))?;

        buf.set_start(buf.get_start() + EXTRA_LEN);
        buf.set_len(buf.len() - TAG_LEN);
        Ok(())
    }

    pub fn create_ecdh_keypair() -> (EphemeralPrivateKey, UnparsedPublicKey<Vec<u8>>) {
        let rand = SystemRandom::new();
        let ecdh_private_key = EphemeralPrivateKey::generate(&X25519, &rand).unwrap();
        let public_key = ecdh_private_key.compute_public_key().unwrap();
        let mut vec = vec![];
        vec.extend_from_slice(public_key.as_ref());
        let ecdh_public_key = UnparsedPublicKey::new(&X25519, vec);
        (ecdh_private_key, ecdh_public_key)
    }

    fn derive_master_key(
        algo: &'static ring::aead::Algorithm,
        privk: EphemeralPrivateKey,
        pubk: &UnparsedPublicKey<Vec<u8>>,
    ) -> LessSafeKey {
        agree_ephemeral(privk, pubk, |k| {
            UnboundKey::new(algo, &k[..algo.key_len()])
                .map(LessSafeKey::new)
                .unwrap()
        })
        .unwrap()
    }
}
