use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

use super::{DbPage, PageRequest};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbCluster {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub security_key: Option<String>,
    pub enable: bool,
    pub enable_default_site: bool,
    pub enable_white_list: bool,
    pub enable_block_list: bool,
    pub description: Option<String>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbCluster {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbCluster,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_cluster",
            table,
            [id, name, security_key, description, gmt_create, gmt_modified],
            [enable, enable_default_site, enable_white_list, enable_block_list]
        )
    }

    pub async fn select_all<'e, E: MySqlExecutor<'e>>(e: E) -> sqlx::Result<Vec<TbCluster>> {
        sqlx::query_as::<_, TbCluster>("SELECT * FROM tb_cluster ORDER BY id ASC")
            .fetch_all(e)
            .await
    }

    pub async fn select_by_name<'e, E: MySqlExecutor<'e>>(
        e: E,
        name: &str,
    ) -> sqlx::Result<Option<TbCluster>> {
        sqlx::query_as::<_, TbCluster>("SELECT * FROM tb_cluster WHERE name = ?")
            .bind(name)
            .fetch_optional(e)
            .await
    }

    pub async fn select_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        id: i64,
    ) -> sqlx::Result<Option<TbCluster>> {
        sqlx::query_as::<_, TbCluster>("SELECT * FROM tb_cluster WHERE id = ?")
            .bind(id)
            .fetch_optional(e)
            .await
    }

    pub async fn delete_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_cluster WHERE id = ?")
            .bind(id)
            .execute(e)
            .await
    }

    pub async fn delete_by_name<'e, E: MySqlExecutor<'e>>(
        e: E,
        name: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_cluster WHERE name = ?")
            .bind(name)
            .execute(e)
            .await
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_page(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
    ) -> sqlx::Result<DbPage<TbCluster>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) = sqlx::query_as("SELECT count(1) FROM tb_cluster")
                .fetch_one(pool)
                .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbCluster>(
            "SELECT * FROM tb_cluster ORDER BY id DESC LIMIT ?,?",
        )
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }

    /// update: None fields are not updated
    pub async fn update_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbCluster,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_cluster",
            table,
            "id = ?",
            [name, security_key, description, gmt_create, gmt_modified],
            [enable, enable_default_site, enable_white_list, enable_block_list],
            [id]
        )
    }

    /// update: None fields are not updated
    pub async fn update_by_name<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbCluster,
        name: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_cluster",
            table,
            "name = ?",
            [name, security_key, description, gmt_create, gmt_modified],
            [enable, enable_default_site, enable_white_list, enable_block_list],
            [name]
        )
    }
}
