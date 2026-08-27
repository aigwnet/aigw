use axum::extract::{Path, Query, State};

use crate::{
    server::http::{ApiContext, ApiData, ApiError, ApiResponseResult, Pagination},
    service::{Page, Server, find_aigw_by_page},
    storage::PageRequest,
};

pub(crate) struct HttpApiServer {}

impl HttpApiServer {
    pub async fn query_by_page(
        Path(cluster): Path<String>,
        Query(page): Query<Pagination>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Page<Server>> {
        let page_request = PageRequest::new(page.page, page.page_size);
        let r = find_aigw_by_page(&context.database_client.rb, &page_request, &cluster)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(r)))
    }
}
