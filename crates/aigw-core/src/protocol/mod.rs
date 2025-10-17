pub(crate) mod close;
pub(crate) mod data;
pub(crate) mod frame;
pub(crate) mod handshake;
pub(crate) mod heartbeat;

pub(crate) mod pb {
    include!(concat!(env!("OUT_DIR"), "/protocol.rs"));
}

use ring::aead::{AES_128_GCM, AES_256_GCM, CHACHA20_POLY1305};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Aes128Gcm,
    Aes256Gcm,
    Chacha20Poly1305,
}

impl Algorithm {
    pub fn code(&self) -> i32 {
        match self {
            Algorithm::Aes128Gcm => 0,
            Algorithm::Aes256Gcm => 1,
            Algorithm::Chacha20Poly1305 => 2,
        }
    }
}

impl From<&Algorithm> for &ring::aead::Algorithm {
    fn from(value: &Algorithm) -> Self {
        match value {
            Algorithm::Aes128Gcm => &AES_128_GCM,
            Algorithm::Aes256Gcm => &AES_256_GCM,
            Algorithm::Chacha20Poly1305 => &CHACHA20_POLY1305,
        }
    }
}

impl TryFrom<i32> for Algorithm {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Algorithm::Aes128Gcm),
            1 => Ok(Algorithm::Aes256Gcm),
            2 => Ok(Algorithm::Chacha20Poly1305),
            _ => Err(anyhow::anyhow!("Algorithm not {} supported", value)),
        }
    }
}

impl TryFrom<&pb::Algorithm> for Algorithm {
    type Error = anyhow::Error;

    fn try_from(value: &pb::Algorithm) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

impl TryFrom<&Algorithm> for pb::Algorithm {
    type Error = anyhow::Error;

    fn try_from(value: &Algorithm) -> Result<Self, Self::Error> {
        value.try_into()
    }
}
