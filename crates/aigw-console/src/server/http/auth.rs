use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use anyhow::anyhow;
use axum::{
    Json,
    extract::{ConnectInfo, FromRequestParts, Request, State},
    http::{HeaderValue, StatusCode, request::Parts},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sha::{
    sha256,
    utils::{Digest, DigestExt},
};

use crate::{
    server::http::{ApiContext, ApiData, ApiError, ApiResponseResult},
    service::{login, token_validate},
    storage::db::DatabaseClient,
};

pub(crate) struct Auth;

#[derive(Serialize, Deserialize)]
pub(crate) struct LoginForm {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Token {
    reset: bool,
    token: String,
}

pub struct AuthHandler {
    database_client: Arc<DatabaseClient>,
}

impl AuthHandler {
    pub fn new(database_client: Arc<DatabaseClient>) -> Self {
        Self { database_client }
    }

    pub async fn validate(&self, token: &str) -> (bool, Option<String>, Option<String>) {
        (token_validate(&self.database_client.rb, token).await).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ExtractIp(Option<IpAddr>);

impl<S> FromRequestParts<S> for ExtractIp
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(ip) = parts.headers.get("x-forwarded-for")
            && let Ok(ip) = ip.to_str()
        {
            for part in ip.split(',') {
                let ip_str = part.trim();
                if let Ok(ip) = IpAddr::from_str(ip_str) {
                    return Ok(ExtractIp(Some(ip)));
                }
            }
        }
        Ok(ExtractIp(None))
    }
}

#[derive(Clone, Debug)]
pub struct ExtractUser(pub Option<String>, pub Option<String>);

impl<S> FromRequestParts<S> for ExtractUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .headers
            .get("x-user-name")
            .and_then(|user| user.to_str().ok())
            .map(|s| s.to_string());
        let email = parts
            .headers
            .get("x-user-email")
            .and_then(|email| email.to_str().ok())
            .map(|s| s.to_string());
        Ok(ExtractUser(user, email))
    }
}

impl Auth {
    pub async fn auth(
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        ExtractIp(ip): ExtractIp,
        State(context): State<ApiContext>,
        Json(login_form): Json<LoginForm>,
    ) -> ApiResponseResult<Token> {
        let client_ip = {
            if let Some(ip) = ip {
                ip.to_string()
            } else {
                addr.ip().to_string()
            }
        };
        let mut sha = sha256::Sha256::default();
        let s = "".to_owned()
            + login_form.username.as_str()
            + "-"
            + login_form.password.as_str()
            + "-"
            + client_ip.as_str();
        sha.digest(s.as_bytes());
        let token = hex::encode(sha.to_bytes());

        let r = login(
            &context.database_client.rb,
            login_form.username.as_str(),
            login_form.password.as_str(),
            client_ip.as_str(),
            token.as_str(),
        )
        .await;
        if let Ok((r, reset)) = r
            && r
        {
            let token = Token { token, reset };
            return Ok(ApiData(Some(token)));
        }
        Err(ApiError::BasicError(anyhow!(
            "The username or password is incorrect. Please try again."
        )))
    }

    pub async fn logout() -> ApiResponseResult<bool> {
        Ok(ApiData(Some(true)))
    }

    pub async fn auth_codes(
        ExtractUser(_user, _email): ExtractUser,
        State(_context): State<ApiContext>,
    ) -> ApiResponseResult<Vec<String>> {
        Ok(ApiData(Some(vec![])))
    }

    pub async fn auth_filter(
        State(auth): State<Arc<AuthHandler>>,
        mut req: Request,
        next: axum::middleware::Next,
    ) -> Result<impl IntoResponse, ApiError> {
        if let Some(authorization) = req.headers().get("authorization")
            && let Ok(authorization) = authorization.to_str()
            && let Some(token_str) = authorization.strip_prefix("Bearer ")
        {
            let (r, user, email) = auth.validate(token_str).await;
            if r {
                let user = HeaderValue::from_str(user.map_or("".to_string(), |u| u).as_str())
                    .map_err(|e| ApiError::BasicError(anyhow!(e)))?;
                let email = HeaderValue::from_str(email.map_or("".to_string(), |u| u).as_str())
                    .map_err(|e| ApiError::BasicError(anyhow!(e)))?;
                req.headers_mut().append("x-user-name", user);
                req.headers_mut().append("x-user-email", email);
                return Ok(next.run(req).await);
            }
        }
        Err(ApiError::AuthenticationError)
    }
}
