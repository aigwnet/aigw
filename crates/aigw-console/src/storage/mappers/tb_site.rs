use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

use super::{DbPage, PageRequest};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbSite {
    pub id: Option<i64>,
    pub cluster_name: Option<String>,
    pub name: Option<String>,
    pub alt_names: Option<String>,
    pub root_dir: Option<String>,
    pub auto_index: bool,
    pub tls_on: bool,
    pub tls_enforce: bool,
    pub acme_on: bool,
    pub tls_cert: Option<String>,
    pub tls_cert_start_date: Option<OffsetDateTime>,
    pub tls_cert_end_date: Option<OffsetDateTime>,
    pub tls_private_key: Option<String>,
    pub rate_limit: Option<i64>,
    pub rate_limit_unit: Option<i64>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbSite {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbSite,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_site",
            table,
            [
                id,
                cluster_name,
                name,
                alt_names,
                root_dir,
                tls_cert,
                tls_cert_start_date,
                tls_cert_end_date,
                tls_private_key,
                rate_limit,
                rate_limit_unit,
                gmt_create,
                gmt_modified
            ],
            [auto_index, tls_on, tls_enforce, acme_on]
        )
    }

    pub async fn select_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        id: i64,
    ) -> sqlx::Result<Option<TbSite>> {
        sqlx::query_as::<_, TbSite>("SELECT * FROM tb_site WHERE id = ?")
            .bind(id)
            .fetch_optional(e)
            .await
    }

    pub async fn select_by_name<'e, E: MySqlExecutor<'e>>(
        e: E,
        name: &str,
    ) -> sqlx::Result<Option<TbSite>> {
        sqlx::query_as::<_, TbSite>("SELECT * FROM tb_site WHERE name = ?")
            .bind(name)
            .fetch_optional(e)
            .await
    }

    pub async fn delete_by_name<'e, E: MySqlExecutor<'e>>(
        e: E,
        name: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_site WHERE name = ?")
            .bind(name)
            .execute(e)
            .await
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_page(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
        cluster_name: &str,
    ) -> sqlx::Result<DbPage<TbSite>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) =
                sqlx::query_as("SELECT count(1) FROM tb_site WHERE cluster_name = ?")
                    .bind(cluster_name)
                    .fetch_one(pool)
                    .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbSite>(
            "SELECT * FROM tb_site WHERE cluster_name = ? ORDER BY id DESC LIMIT ?,?",
        )
        .bind(cluster_name)
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_page_with_acme(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
    ) -> sqlx::Result<DbPage<TbSite>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) = sqlx::query_as("SELECT count(1) FROM tb_site WHERE acme_on=1")
                .fetch_one(pool)
                .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbSite>(
            "SELECT * FROM tb_site WHERE acme_on=1 ORDER BY id DESC LIMIT ?,?",
        )
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }

    /// update: None fields are not updated
    pub async fn update_by_name<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbSite,
        name: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_site",
            table,
            "name = ?",
            [
                cluster_name,
                name,
                alt_names,
                root_dir,
                tls_cert,
                tls_cert_start_date,
                tls_cert_end_date,
                tls_private_key,
                rate_limit,
                rate_limit_unit,
                gmt_create,
                gmt_modified
            ],
            [auto_index, tls_on, tls_enforce, acme_on],
            [name]
        )
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_acme_cert_about_to_expire(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
    ) -> sqlx::Result<DbPage<TbSite>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) = sqlx::query_as(
                "SELECT count(1) FROM tb_site WHERE acme_on=1 AND tls_cert_end_date < DATE_ADD(NOW(), INTERVAL 30 DAY)",
            )
            .fetch_one(pool)
            .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbSite>(
            "SELECT * FROM tb_site WHERE acme_on=1 AND tls_cert_end_date < DATE_ADD(NOW(), INTERVAL 30 DAY) ORDER BY ID DESC LIMIT ?,?",
        )
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }
}
