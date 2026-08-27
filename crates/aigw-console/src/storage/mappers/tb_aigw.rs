use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

use super::{DbPage, PageRequest};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbAigw {
    pub id: Option<i64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub version: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_arch: Option<String>,
    pub cpu_name: Option<String>,
    pub cpu_vendor: Option<String>,
    pub cpu_frequency: Option<i64>,
    pub cpu_nums: Option<i32>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbAigw {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbAigw,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_aigw",
            table,
            [id, cluster_name, ip, version, os_name, os_version, os_arch, cpu_name, cpu_vendor, cpu_frequency, cpu_nums, gmt_create, gmt_modified],
            []
        )
    }

    /// update: None fields are not updated
    pub async fn update_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbAigw,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_aigw",
            table,
            "id = ?",
            [cluster_name, ip, version, os_name, os_version, os_arch, cpu_name, cpu_vendor, cpu_frequency, cpu_nums, gmt_create, gmt_modified],
            [],
            [id]
        )
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_by_page(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
        cluster_name: &str,
    ) -> sqlx::Result<DbPage<TbAigw>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) =
                sqlx::query_as("SELECT count(1) FROM tb_aigw WHERE cluster_name = ?")
                    .bind(cluster_name)
                    .fetch_one(pool)
                    .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbAigw>(
            "SELECT * FROM tb_aigw WHERE cluster_name = ? ORDER BY id DESC LIMIT ?,?",
        )
        .bind(cluster_name)
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }

    pub async fn select_by_cluster_name_and_ip<'e, E: MySqlExecutor<'e>>(
        e: E,
        cluster_name: &str,
        ip: &str,
    ) -> sqlx::Result<Option<TbAigw>> {
        sqlx::query_as::<_, TbAigw>("SELECT * FROM tb_aigw WHERE cluster_name = ? and ip = ?")
            .bind(cluster_name)
            .bind(ip)
            .fetch_optional(e)
            .await
    }
}
