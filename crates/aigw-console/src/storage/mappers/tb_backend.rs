use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbBackend {
    pub id: Option<i64>,
    pub location_id: Option<i64>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbBackend {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbBackend,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_backend",
            table,
            [id, location_id, host, port, gmt_create, gmt_modified],
            []
        )
    }

    pub async fn select_by_location_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        location_id: i64,
    ) -> sqlx::Result<Vec<TbBackend>> {
        sqlx::query_as::<_, TbBackend>("SELECT * FROM tb_backend WHERE location_id = ?")
            .bind(location_id)
            .fetch_all(e)
            .await
    }

    pub async fn delete_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_backend WHERE id = ?")
            .bind(id)
            .execute(e)
            .await
    }
}
