use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbSession {
    pub id: Option<i64>,
    pub user: Option<String>,
    pub email: Option<String>,
    pub login_ip: Option<String>,
    pub token: Option<String>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbSession {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbSession,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_session",
            table,
            [id, user, email, login_ip, token, gmt_create, gmt_modified],
            []
        )
    }

    pub async fn select_by_token<'e, E: MySqlExecutor<'e>>(
        e: E,
        token: &str,
    ) -> sqlx::Result<Option<TbSession>> {
        sqlx::query_as::<_, TbSession>("SELECT * FROM tb_session WHERE token = ?")
            .bind(token)
            .fetch_optional(e)
            .await
    }

    /// update: None fields are not updated
    pub async fn update_by_token<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbSession,
        token: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_session",
            table,
            "token = ?",
            [user, email, login_ip, token, gmt_create, gmt_modified],
            [],
            [token]
        )
    }
}
