use serde::{Deserialize, Serialize};
use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TbUser {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub real_name: Option<String>,
    pub ext_info: Option<String>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbUser {
    /// insert: None fields are skipped (DB defaults apply)
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbUser,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_insert!(
            e,
            "tb_user",
            table,
            [id, name, email, password, real_name, ext_info, gmt_create, gmt_modified],
            []
        )
    }

    pub async fn select_by_name<'e, E: MySqlExecutor<'e>>(
        e: E,
        name: &str,
    ) -> sqlx::Result<Option<TbUser>> {
        sqlx::query_as::<_, TbUser>("SELECT * FROM tb_user WHERE name = ?")
            .bind(name)
            .fetch_optional(e)
            .await
    }

    pub async fn select_by_email<'e, E: MySqlExecutor<'e>>(
        e: E,
        email: &str,
    ) -> sqlx::Result<Option<TbUser>> {
        sqlx::query_as::<_, TbUser>("SELECT * FROM tb_user WHERE email = ?")
            .bind(email)
            .fetch_optional(e)
            .await
    }

    /// update: None fields are not updated
    pub async fn update_by_name<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbUser,
        name: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_user",
            table,
            "name = ?",
            [name, email, password, real_name, ext_info, gmt_create, gmt_modified],
            [],
            [name]
        )
    }

    /// update: None fields are not updated
    pub async fn update_by_email<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbUser,
        email: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        crate::sqlx_update!(
            e,
            "tb_user",
            table,
            "email = ?",
            [name, email, password, real_name, ext_info, gmt_create, gmt_modified],
            [],
            [email]
        )
    }

    pub async fn select_default_user<'e, E: MySqlExecutor<'e>>(
        e: E,
    ) -> sqlx::Result<Option<TbUser>> {
        sqlx::query_as::<_, TbUser>(
            "SELECT * FROM tb_user WHERE email IS NOT NULL ORDER BY id ASC LIMIT 1",
        )
        .fetch_optional(e)
        .await
    }
}
