use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbLock {
    pub id: Option<i64>,
    pub lock_key: Option<String>,
    pub host: Option<String>,
    pub expires_at: Option<OffsetDateTime>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbLock {
    pub async fn delete_by_key<'e, E: MySqlExecutor<'e>>(
        e: E,
        key: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_lock WHERE lock_key = ?")
            .bind(key)
            .execute(e)
            .await
    }
}
