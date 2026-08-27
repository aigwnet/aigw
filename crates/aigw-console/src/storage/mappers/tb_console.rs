use serde::{Deserialize, Serialize};
use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

use super::{DbPage, PageRequest};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TbConsole {
    pub id: Option<i64>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub last_active_time: Option<OffsetDateTime>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbConsole {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbConsole,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_console",
            table,
            [id, host, port, last_active_time, gmt_create, gmt_modified],
            []
        )
    }

    /// update: None fields are not updated
    pub async fn update_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbConsole,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_console",
            table,
            "id = ?",
            [host, port, last_active_time, gmt_create, gmt_modified],
            [],
            [id]
        )
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_by_page(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
    ) -> sqlx::Result<DbPage<TbConsole>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) = sqlx::query_as("SELECT count(1) FROM tb_console")
                .fetch_one(pool)
                .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbConsole>(
            "SELECT * FROM tb_console ORDER BY id DESC LIMIT ?,?",
        )
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }

    pub async fn select_by_host_port<'e, E: MySqlExecutor<'e>>(
        e: E,
        host: &str,
        port: i32,
    ) -> sqlx::Result<Option<TbConsole>> {
        sqlx::query_as::<_, TbConsole>("SELECT * FROM tb_console WHERE host = ? and port = ?")
            .bind(host)
            .bind(port)
            .fetch_optional(e)
            .await
    }
}
