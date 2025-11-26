use serde::{Deserialize, Serialize};

/// Represents an ACME HTTP-01 challenge token used for domain validation.
/// 
/// This struct holds the necessary data to respond to an ACME (Automated Certificate
/// Management Environment) HTTP-01 challenge. The ACME server will request a specific
/// token from a well-known URL on the domain being validated, and the client must serve
/// the corresponding proof (typically a JWK thumbprint-based key authorization) at that URL.
/// 
/// Fields:
/// - `host`: The domain name for which the challenge is issued.
/// - `token`: The unique token provided by the ACME server for this challenge.
/// - `proof`: The key authorization string that must be served at
///   `http://{host}/.well-known/acme-challenge/{token}` to prove control over the domain.
#[derive(Serialize, Deserialize, Clone)]
pub struct AcmeToken {
    pub host: String,
    pub token: String,
    pub proof: String,
}
