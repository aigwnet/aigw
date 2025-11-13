use axum::{
    Json,
    extract::{Path, Query, State},
};
use rbatis::PageRequest;

use crate::{
    server::http::{ApiContext, ApiData, ApiError, ApiResponseResult, Pagination},
    service::{ClusterIpCidr, Page, add_new_cluster_ip, find_ip_cidr_by_page},
};

pub(crate) struct HttpApiSecurity {}

impl HttpApiSecurity {
    pub async fn add_cluster_ip_list(
        State(context): State<ApiContext>,
        Json(ip): Json<ClusterIpCidr>,
    ) -> ApiResponseResult<()> {
        let change_log = add_new_cluster_ip(&context.database_client.rb, &ip)
            .await
            .map_err(ApiError::from)?;
        let _ = context.sender.send(change_log).await;
        Ok(ApiData(None))
    }

    pub async fn query_cluster_ip_list(
        Path((cluster_name, r#type)): Path<(String, u8)>,
        Query(page): Query<Pagination>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Page<ClusterIpCidr>> {
        let mut page_request = PageRequest::new(page.page, page.page_size);
        page_request = page_request.set_do_count(true);
        let data = find_ip_cidr_by_page(
            &context.database_client.rb,
            &page_request,
            &cluster_name,
            r#type,
        )
        .await
        .map_err(ApiError::from)?;
        Ok(ApiData(Some(data)))
    }
}
