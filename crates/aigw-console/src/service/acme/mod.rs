mod account;
mod authorization;
mod directory;
mod error;
mod jws;
mod order;

pub(crate) use account::Account;
pub(crate) use account::AccountBuilder;
pub(crate) use authorization::AuthorizationStatus;
pub(crate) use authorization::ChallengeStatus;
pub(crate) use directory::DirectoryBuilder;
pub(crate) use order::OrderBuilder;
pub(crate) use order::OrderStatus;

use base64::Engine;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

/// This is a identifier for a resource that the ACME server
/// can provision certificates for (a domain).
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Identifier {
    /// The type of identifier.
    pub r#type: String,
    /// The identifier itself.
    pub value: String,
}

pub(crate) fn b64(data: &[u8]) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(data)
}
