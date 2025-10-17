use crate::LogPoint;

use super::{Algorithm, pb};

#[derive(Clone, Debug)]
pub struct HandshakeInfo {
    pub ip: String,
    pub cluster: String,
    pub version: String,
    pub os_name: String,
    pub os_version: String,
    pub os_arch: String,
    pub cpu_name: String,
    pub cpu_vendor: String,
    pub cpu_frequency: u64,
    pub cpu_nums: u32,
}

#[derive(Clone, Debug)]
pub struct HandshakeRequest {
    pub public_key_salt: Vec<u8>,
    pub public_key_hash: Vec<u8>,
    pub public_key_data: Vec<u8>,
    pub log_points: Vec<LogPoint>,
    pub info: HandshakeInfo,
}

#[derive(Clone, Debug)]
pub struct HandshakeResponse {
    pub public_key_salt: Vec<u8>,
    pub public_key_hash: Vec<u8>,
    pub public_key_data: Vec<u8>,
    pub algorithm: Algorithm,
}

impl TryFrom<pb::HandshakeRequest> for HandshakeRequest {
    type Error = anyhow::Error;

    fn try_from(value: pb::HandshakeRequest) -> Result<Self, Self::Error> {
        Ok(HandshakeRequest {
            public_key_salt: value.public_key_salt,
            public_key_hash: value.public_key_hash,
            public_key_data: value.public_key_data,
            log_points: value.log_points.into_iter().map(|l| l.into()).collect(),
            info: HandshakeInfo {
                ip: value.ip,
                cluster: value.cluster,
                version: value.version,
                os_name: value.os_name,
                os_version: value.os_version,
                os_arch: value.os_arch,
                cpu_name: value.cpu_name,
                cpu_vendor: value.cpu_vendor,
                cpu_frequency: value.cpu_frequency,
                cpu_nums: value.cpu_nums,
            },
        })
    }
}

impl TryFrom<pb::HandshakeResponse> for HandshakeResponse {
    type Error = anyhow::Error;

    fn try_from(value: pb::HandshakeResponse) -> Result<Self, Self::Error> {
        Ok(HandshakeResponse {
            public_key_salt: value.public_key_salt,
            public_key_hash: value.public_key_hash,
            public_key_data: value.public_key_data,
            algorithm: value.algorithm.try_into()?,
        })
    }
}
