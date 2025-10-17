use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use crate::{
    server::http::{ApiContext, ApiData, ApiError, ApiResponseResult},
    service::{
        AnalyticsMonitorItem, AnalyticsTrafficItem, ExtInfo, get_analytics_monitor,
        get_analytics_traffic, get_analytics_traffic_1day, get_analytics_traffic_1month,
        get_analytics_traffic_ext_info_1month,
    },
};

pub(crate) struct HttpApiAnalytics {}

#[derive(Serialize, Deserialize)]
pub struct Traffic {
    data_latest_30: Vec<AnalyticsTrafficItem>,
    data_1day: Vec<AnalyticsTrafficItem>,
    data_1month: Vec<AnalyticsTrafficItem>,
}

impl HttpApiAnalytics {
    pub async fn analytics_traffic(
        Path(cluster): Path<String>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Traffic> {
        let data_latest_30 =
            get_analytics_traffic(&context.database_client.rb, &cluster, 30).await?;

        let data_1day = get_analytics_traffic_1day(&context.database_client.rb, &cluster)
            .await
            .map_err(ApiError::from)?;

        let data_1month = get_analytics_traffic_1month(&context.database_client.rb, &cluster)
            .await
            .map_err(ApiError::from)?;

        Ok(ApiData(Some(Traffic {
            data_latest_30,
            data_1day,
            data_1month,
        })))
    }

    pub async fn analytics_traffic_ext(
        Path(cluster): Path<String>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<ExtInfo> {
        let data =
            get_analytics_traffic_ext_info_1month(&context.database_client.rb, &cluster).await?;
        Ok(ApiData(Some(data)))
    }

    pub async fn analytics_monitor(
        Path(cluster): Path<String>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Vec<AnalyticsMonitorItem>> {
        let data = get_analytics_monitor(&context.database_client.rb, &cluster, 30)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(data)))
    }
}
