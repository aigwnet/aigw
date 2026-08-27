use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbLocation {
    pub id: Option<i64>,
    pub site_id: Option<i64>,
    pub location: Option<String>,
    pub proxy: Option<i8>,
    pub protocol: Option<String>,
    pub sni: Option<String>,
    pub client_max_body_size: Option<i64>,
    pub connection_timeout: Option<i32>,
    pub read_timeout: Option<i32>,
    pub write_timeout: Option<i32>,
    pub idle_timeout: Option<i32>,
    pub rewrite: Option<String>,
    pub http_version: Option<String>,
    pub proxy_set_headers: Option<String>,
    pub proxy_add_headers: Option<String>,
    pub proxy_remove_headers: Option<String>,
    pub response_set_headers: Option<String>,
    pub response_add_headers: Option<String>,
    pub response_remove_headers: Option<String>,
    pub root_dir: Option<String>,
    pub auto_index: Option<i8>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbLocation {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbLocation,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_location",
            table,
            [
                id,
                site_id,
                location,
                proxy,
                protocol,
                sni,
                client_max_body_size,
                connection_timeout,
                read_timeout,
                write_timeout,
                idle_timeout,
                rewrite,
                http_version,
                proxy_set_headers,
                proxy_add_headers,
                proxy_remove_headers,
                response_set_headers,
                response_add_headers,
                response_remove_headers,
                root_dir,
                auto_index,
                gmt_create,
                gmt_modified
            ],
            []
        )
    }

    pub async fn select_by_site_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        site_id: i64,
    ) -> sqlx::Result<Vec<TbLocation>> {
        sqlx::query_as::<_, TbLocation>("SELECT * FROM tb_location WHERE site_id = ?")
            .bind(site_id)
            .fetch_all(e)
            .await
    }

    pub async fn select_by_site_id_and_location<'e, E: MySqlExecutor<'e>>(
        e: E,
        site_id: i64,
        location: &str,
    ) -> sqlx::Result<Option<TbLocation>> {
        sqlx::query_as::<_, TbLocation>(
            "SELECT * FROM tb_location WHERE site_id = ? AND location = ?",
        )
        .bind(site_id)
        .bind(location)
        .fetch_optional(e)
        .await
    }

    pub async fn select_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        id: i64,
    ) -> sqlx::Result<Option<TbLocation>> {
        sqlx::query_as::<_, TbLocation>("SELECT * FROM tb_location WHERE id = ?")
            .bind(id)
            .fetch_optional(e)
            .await
    }

    /// update: None fields are not updated
    pub async fn update_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbLocation,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_location",
            table,
            "id = ?",
            [
                site_id,
                location,
                proxy,
                protocol,
                sni,
                client_max_body_size,
                connection_timeout,
                read_timeout,
                write_timeout,
                idle_timeout,
                rewrite,
                http_version,
                proxy_set_headers,
                proxy_add_headers,
                proxy_remove_headers,
                response_set_headers,
                response_add_headers,
                response_remove_headers,
                root_dir,
                auto_index,
                gmt_create,
                gmt_modified
            ],
            [],
            [id]
        )
    }

    pub async fn delete_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_location WHERE id = ?")
            .bind(id)
            .execute(e)
            .await
    }
}
