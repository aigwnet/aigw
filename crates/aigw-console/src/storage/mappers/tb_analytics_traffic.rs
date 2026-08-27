use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

use super::{DbPage, PageRequest};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbAnalyticsTraffic {
    pub id: Option<i64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub tls: Option<i64>,
    pub pv: Option<i64>,
    pub http_country: Option<String>,
    pub http_code: Option<String>,
    pub http_source: Option<String>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbAnalyticsTraffic {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbAnalyticsTraffic,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_analytics_traffic",
            table,
            [id, cluster_name, ip, tls, pv, http_country, http_code, http_source, gmt_create, gmt_modified],
            []
        )
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_page_by_cluster_and_time(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
        cluster_name: &str,
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
    ) -> sqlx::Result<DbPage<TbAnalyticsTraffic>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) = sqlx::query_as(
                "SELECT count(1) FROM tb_analytics_traffic WHERE cluster_name = ? and gmt_create >= ? and gmt_create < ?",
            )
            .bind(cluster_name)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(pool)
            .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbAnalyticsTraffic>(
            "SELECT * FROM tb_analytics_traffic WHERE cluster_name = ? and gmt_create >= ? and gmt_create < ? ORDER BY id DESC LIMIT ?,?",
        )
        .bind(cluster_name)
        .bind(start_time)
        .bind(end_time)
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }

    pub async fn delete_by_gmt_create<'e, E: MySqlExecutor<'e>>(
        e: E,
        gmt_create: OffsetDateTime,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_analytics_traffic WHERE gmt_create < ?")
            .bind(gmt_create)
            .execute(e)
            .await
    }
}
