use std::{io::Error, net::SocketAddr, sync::Arc};

use aigw_core::ChangeLog;
use axum::{
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;

use crate::{
    AigwConsoleConfig, DatabaseClient,
    server::http::{
        analytics::HttpApiAnalytics,
        auth::{Auth, AuthHandler},
        cluster::HttpApiCluster,
        server::HttpApiServer,
        site::HttpApiSite,
        user::User,
    },
};

mod analytics;
mod auth;
mod cluster;
mod server;
mod site;
mod user;

#[derive(Clone)]
struct ApiContext {
    sender: Sender<ChangeLog>,
    database_client: Arc<DatabaseClient>,
}

#[derive(Serialize, Deserialize)]
pub struct Pagination {
    pub page: u64,
    pub page_size: u64,
}

pub async fn run(
    sender: Sender<ChangeLog>,
    database_client: Arc<DatabaseClient>,
    config: Arc<AigwConsoleConfig>,
) -> anyhow::Result<()> {
    let api_context = ApiContext {
        sender,
        database_client: database_client.clone(),
    };

    let auth_handler = Arc::new(AuthHandler::new(database_client));

    let auth_layer = axum::middleware::from_fn_with_state(auth_handler.clone(), Auth::auth_filter);

    let app = Router::new()
        .route("/api/v1/auth/login", post(Auth::auth))
        .route("/api/v1/auth/logout", post(Auth::logout))
        .route(
            "/api/v1/auth/codes",
            get(Auth::auth_codes).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/user/info",
            get(User::profile).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/user/profile/{user}",
            put(User::update_profile).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/user/profile/{user}/password",
            put(User::update_password).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/clusters",
            post(HttpApiCluster::cluster_add).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/clusters/{name}",
            put(HttpApiCluster::cluster_modify).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/clusters/{name}",
            get(HttpApiCluster::cluster_detail).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/clusters/{name}",
            delete(HttpApiCluster::cluster_delete).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/clusters/",
            get(HttpApiCluster::query_by_page).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/clusters/all",
            get(HttpApiCluster::clusters).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/sites",
            post(HttpApiSite::add).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/sites/{name}",
            put(HttpApiSite::update).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/sites/{name}",
            get(HttpApiSite::query).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/sites/{name}",
            delete(HttpApiSite::delete).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/sites/page/{cluster_name}",
            get(HttpApiSite::query_by_page).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/servers/{cluster}",
            get(HttpApiServer::query_by_page).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/analytics/monitor/{cluster}",
            get(HttpApiAnalytics::analytics_monitor).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/analytics/monitor/{cluster}/{ip}",
            get(HttpApiAnalytics::analytics_monitor_server).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/analytics/traffic/{cluster}",
            get(HttpApiAnalytics::analytics_traffic).layer(auth_layer.clone()),
        )
        .route(
            "/api/v1/analytics/traffic/{cluster}/ext",
            get(HttpApiAnalytics::analytics_traffic_ext).layer(auth_layer.clone()),
        )
        .with_state(api_context)
        .fallback_service(ServeDir::new(config.server.ui.as_ref().unwrap()))
        .layer(TraceLayer::new_for_http())
        .into_make_service_with_connect_info::<SocketAddr>();

    let addr = "127.0.0.1:".to_string() + config.server.http.port.to_string().as_str();
    info!(
        "Http server listening on: {}. ui directory: {:?}",
        addr, &config.server.ui
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Serialize)]
struct ApiResult<T: Serialize> {
    code: i32,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<T>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

pub type ApiResponseResult<T> = Result<ApiData<T>, ApiError>;
pub struct ApiData<T: Serialize>(Option<T>);

impl<T: Serialize> IntoResponse for ApiData<T> {
    fn into_response(self) -> axum::response::Response {
        let r = ApiResult::<T> {
            code: 0,
            data: self.0,
            message: None,
        };
        Json(r).into_response()
    }
}

#[derive(Debug)]
pub enum ApiError {
    BasicError(anyhow::Error),
    AuthenticationError,
}

impl From<Error> for ApiError {
    fn from(value: Error) -> Self {
        ApiError::BasicError(anyhow::anyhow!(value))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        ApiError::BasicError(anyhow::anyhow!(value))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        ApiError::BasicError(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::BasicError(error) => {
                let r = ApiResult::<String> {
                    code: -1,
                    data: None,
                    message: Some(error.to_string()),
                };
                Json(r).into_response()
            }
            ApiError::AuthenticationError => {
                (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
            }
        }
    }
}
