use axum::extract::{Path, Query, State};
use rbatis::PageRequest;

use crate::{
    server::http::{ApiContext, ApiData, ApiError, ApiResponseResult, Pagination},
    service::{Page, Server, find_server_by_page},
};

pub(crate) struct HttpApiServer {}

impl HttpApiServer {
    pub async fn query_by_page(
        Path(cluster): Path<String>,
        Query(page): Query<Pagination>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Page<Server>> {
        let mut page_request = PageRequest::new(page.page, page.page_size);
        page_request = page_request.set_do_count(true);
        let r = find_server_by_page(&context.database_client.rb, &page_request, &cluster)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(r)))
    }
}
