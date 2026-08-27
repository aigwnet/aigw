use aigw_core::Cluster;
use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{
    server::http::{ApiContext, ApiData, ApiError, ApiResponseResult, Pagination},
    service::{
        Page, add_cluster, delete_cluster, find_all, find_cluster, find_cluster_by_page,
        modify_cluster,
    },
    storage::PageRequest,
};

pub(crate) struct HttpApiCluster {}

impl HttpApiCluster {
    pub async fn add(
        State(context): State<ApiContext>,
        Json(cluster): Json<Cluster>,
    ) -> ApiResponseResult<()> {
        let change_log = add_cluster(&context.database_client.rb, &cluster)
            .await
            .map_err(ApiError::from)?;
        let _ = context.sender.send(change_log).await;
        Ok(ApiData(None))
    }

    pub async fn update(
        Path(name): Path<String>,
        State(context): State<ApiContext>,
        Json(cluster): Json<Cluster>,
    ) -> ApiResponseResult<()> {
        let change_log = modify_cluster(&context.database_client.rb, &cluster, &name)
            .await
            .map_err(ApiError::from)?;
        let _ = context.sender.send(change_log).await;
        Ok(ApiData(None))
    }

    pub async fn query(
        Path(name): Path<String>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Cluster> {
        let cluster = find_cluster(&context.database_client.rb, &name)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(cluster)))
    }

    pub async fn query_all(State(context): State<ApiContext>) -> ApiResponseResult<Vec<Cluster>> {
        let clusters = find_all(&context.database_client.rb)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(clusters)))
    }

    pub async fn query_by_page(
        Query(page): Query<Pagination>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Page<Cluster>> {
        let page_request = PageRequest::new(page.page, page.page_size);
        let r = find_cluster_by_page(&context.database_client.rb, &page_request)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(r)))
    }

    pub async fn delete(
        Path(name): Path<String>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<bool> {
        delete_cluster(&context.database_client.rb, name.as_str())
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(true)))
    }
}
