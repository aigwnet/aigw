use axum::{
    Json,
    extract::{Path, Query, State},
};
use rbatis::PageRequest;
use serde::{Deserialize, Deserializer};

use crate::{
    server::http::{ApiContext, ApiData, ApiError, ApiResponseResult, Pagination},
    service::{
        ClusterIpCidr, ClusterIpCidrList, Page, add_new_cluster_ip, delete_cluster_ip,
        find_ip_cidr_by_page,
    },
};

pub(crate) struct HttpApiSecurity {}

#[derive(Deserialize)]
pub struct DeleteBatch {
    #[serde(deserialize_with = "deserialize_ids")]
    pub ids: Vec<u64>,
}

fn deserialize_ids<'de, D>(deserializer: D) -> Result<Vec<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.split(',')
        .map(|x| x.parse::<u64>().map_err(serde::de::Error::custom))
        .collect()
}

impl HttpApiSecurity {
    pub async fn add_cluster_ip_list(
        State(context): State<ApiContext>,
        Json(list): Json<ClusterIpCidrList>,
    ) -> ApiResponseResult<()> {
        let change_log = add_new_cluster_ip(&context.database_client.rb, &list)
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

    pub async fn delete(
        Path(id): Path<u64>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<bool> {
        let change_log = delete_cluster_ip(&context.database_client.rb, id)
            .await
            .map_err(ApiError::from)?;
        let _ = context.sender.send(change_log).await;
        Ok(ApiData(Some(true)))
    }

    pub async fn delete_batch(
        Query(q): Query<DeleteBatch>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<bool> {
        for id in q.ids {
            let change_log = delete_cluster_ip(&context.database_client.rb, id)
                .await
                .map_err(ApiError::from)?;
            let _ = context.sender.send(change_log).await;
        }

        Ok(ApiData(Some(true)))
    }
}
