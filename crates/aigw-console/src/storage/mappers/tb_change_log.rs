use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

use super::{DbPage, PageRequest};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbChangeLog {
    pub id: Option<i64>,
    pub cluster_name: Option<String>,
    pub log_type: Option<i32>,
    pub log_action: Option<i32>,
    pub data_id: Option<i64>,
    pub data: Option<String>,
    pub expire_second: Option<i32>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbChangeLog {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbChangeLog,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_change_log",
            table,
            [id, cluster_name, log_type, log_action, data_id, data, expire_second, gmt_create, gmt_modified],
            []
        )
    }

    pub async fn delete_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        log_id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_change_log WHERE id = ?")
            .bind(log_id)
            .execute(e)
            .await
    }

    pub async fn delete_expired<'e, E: MySqlExecutor<'e>>(
        e: E,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query(
            "DELETE FROM tb_change_log WHERE expire_second != 0 and TIMESTAMPDIFF(SECOND, gmt_modified, NOW()) > expire_second",
        )
        .execute(e)
        .await
    }

    pub async fn delete_by_data_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        data_id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_change_log WHERE data_id = ?")
            .bind(data_id)
            .execute(e)
            .await
    }

    pub async fn select_by_data_id_and_type<'e, E: MySqlExecutor<'e>>(
        e: E,
        log_type: i32,
        data_id: i64,
    ) -> sqlx::Result<Option<TbChangeLog>> {
        sqlx::query_as::<_, TbChangeLog>(
            "SELECT * FROM tb_change_log WHERE log_type = ? and data_id = ? and (expire_second = 0 or TIMESTAMPDIFF(SECOND, gmt_modified, NOW()) <= expire_second)",
        )
        .bind(log_type)
        .bind(data_id)
        .fetch_optional(e)
        .await
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_by_type(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
        cluster_name: &str,
        log_type: i32,
        log_id: i64,
    ) -> sqlx::Result<DbPage<TbChangeLog>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) = sqlx::query_as(
                "SELECT count(1) FROM tb_change_log WHERE cluster_name = ? and log_type = ? and id > ? and (expire_second = 0 or TIMESTAMPDIFF(SECOND, gmt_modified, NOW()) <= expire_second)",
            )
            .bind(cluster_name)
            .bind(log_type)
            .bind(log_id)
            .fetch_one(pool)
            .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbChangeLog>(
            "SELECT * FROM tb_change_log WHERE cluster_name = ? and log_type = ? and id > ? and (expire_second = 0 or TIMESTAMPDIFF(SECOND, gmt_modified, NOW()) <= expire_second) LIMIT ?,?",
        )
        .bind(cluster_name)
        .bind(log_type)
        .bind(log_id)
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }
}
