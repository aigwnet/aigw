use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

use super::{DbPage, PageRequest};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbAnalyticsMonitorCluster {
    pub id: Option<i64>,
    pub cluster_name: Option<String>,
    pub cpu: Option<f64>,
    pub cpu_current_process: Option<f64>,
    pub cpu_load_one: Option<f64>,
    pub cpu_load_five: Option<f64>,
    pub cpu_load_fifteen: Option<f64>,
    pub mem: Option<f64>,
    pub swap: Option<f64>,
    pub disk: Option<f64>,
    pub io_read: Option<i64>,
    pub io_written: Option<i64>,
    pub net_send: Option<i64>,
    pub net_received: Option<i64>,
    pub rt: Option<i64>,
    pub error: Option<i64>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbAnalyticsMonitorCluster {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbAnalyticsMonitorCluster,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_analytics_monitor_cluster",
            table,
            [id, cluster_name, cpu, cpu_current_process, cpu_load_one, cpu_load_five, cpu_load_fifteen, mem, swap, disk, io_read, io_written, net_send, net_received, rt, error, gmt_create, gmt_modified],
            []
        )
    }

    pub async fn select_by_cluster<'e, E: MySqlExecutor<'e>>(
        e: E,
        cluster_name: &str,
        limit: u64,
    ) -> sqlx::Result<Vec<TbAnalyticsMonitorCluster>> {
        sqlx::query_as::<_, TbAnalyticsMonitorCluster>(
            "SELECT * FROM tb_analytics_monitor_cluster WHERE cluster_name = ? ORDER BY id DESC LIMIT ?",
        )
        .bind(cluster_name)
        .bind(limit)
        .fetch_all(e)
        .await
    }

    pub async fn select_by_cluster_gmt_create<'e, E: MySqlExecutor<'e>>(
        e: E,
        cluster_name: &str,
        gmt_create: OffsetDateTime,
    ) -> sqlx::Result<Option<TbAnalyticsMonitorCluster>> {
        sqlx::query_as::<_, TbAnalyticsMonitorCluster>(
            "SELECT * FROM tb_analytics_monitor_cluster WHERE cluster_name = ? and gmt_create = ?",
        )
        .bind(cluster_name)
        .bind(gmt_create)
        .fetch_optional(e)
        .await
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_page_by_cluster_and_time(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
        cluster_name: &str,
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
    ) -> sqlx::Result<DbPage<TbAnalyticsMonitorCluster>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) = sqlx::query_as(
                "SELECT count(1) FROM tb_analytics_monitor_cluster WHERE cluster_name = ? and gmt_create >= ? and gmt_create < ?",
            )
            .bind(cluster_name)
            .bind(start_time)
            .bind(end_time)
            .fetch_one(pool)
            .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbAnalyticsMonitorCluster>(
            "SELECT * FROM tb_analytics_monitor_cluster WHERE cluster_name = ? and gmt_create >= ? and gmt_create < ? ORDER BY id DESC LIMIT ?,?",
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
}
