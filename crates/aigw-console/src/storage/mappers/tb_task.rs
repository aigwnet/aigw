use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbTask {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub r#type: Option<i32>,
    pub last_time: Option<OffsetDateTime>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbTask {
    /// insert: None fields are skipped (DB defaults apply).
    /// Written manually (not via sqlx_insert!) because `r#type` is a raw
    /// identifier: stringify!(r#type) would emit the wrong column name.
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbTask,
    ) -> sqlx::Result<MySqlQueryResult> {
        let mut cols = String::new();
        let mut vals = String::new();
        if table.id.is_some() {
            cols.push_str("id,");
            vals.push_str("?,");
        }
        if table.name.is_some() {
            cols.push_str("name,");
            vals.push_str("?,");
        }
        if table.r#type.is_some() {
            cols.push_str("type,");
            vals.push_str("?,");
        }
        if table.last_time.is_some() {
            cols.push_str("last_time,");
            vals.push_str("?,");
        }
        if table.gmt_create.is_some() {
            cols.push_str("gmt_create,");
            vals.push_str("?,");
        }
        if table.gmt_modified.is_some() {
            cols.push_str("gmt_modified,");
            vals.push_str("?,");
        }
        cols.pop();
        vals.pop();
        let sql = format!("INSERT INTO tb_task ({}) VALUES ({})", cols, vals);
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        if let Some(v) = &table.id {
            q = q.bind(v);
        }
        if let Some(v) = &table.name {
            q = q.bind(v);
        }
        if let Some(v) = &table.r#type {
            q = q.bind(v);
        }
        if let Some(v) = &table.last_time {
            q = q.bind(v);
        }
        if let Some(v) = &table.gmt_create {
            q = q.bind(v);
        }
        if let Some(v) = &table.gmt_modified {
            q = q.bind(v);
        }
        q.execute(e).await
    }

    pub async fn select_by_name_and_type<'e, E: MySqlExecutor<'e>>(
        e: E,
        name: &str,
        t: i32,
    ) -> sqlx::Result<Option<TbTask>> {
        sqlx::query_as::<_, TbTask>("SELECT * FROM tb_task WHERE name = ? AND type = ?")
            .bind(name)
            .bind(t)
            .fetch_optional(e)
            .await
    }

    /// update: None fields are not updated.
    /// Written manually (not via sqlx_update!) because `r#type` is a raw
    /// identifier: stringify!(r#type) would emit the wrong column name.
    pub async fn update_by_name_and_type<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbTask,
        name: &str,
        t: i32,
    ) -> sqlx::Result<MySqlQueryResult> {
        let mut sets = String::new();
        if table.name.is_some() {
            sets.push_str("name = ?,");
        }
        if table.r#type.is_some() {
            sets.push_str("type = ?,");
        }
        if table.last_time.is_some() {
            sets.push_str("last_time = ?,");
        }
        if table.gmt_create.is_some() {
            sets.push_str("gmt_create = ?,");
        }
        if table.gmt_modified.is_some() {
            sets.push_str("gmt_modified = ?,");
        }
        sets.pop();
        let sql = format!("UPDATE tb_task SET {} WHERE name = ? AND type = ?", sets);
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        if let Some(v) = &table.name {
            q = q.bind(v);
        }
        if let Some(v) = &table.r#type {
            q = q.bind(v);
        }
        if let Some(v) = &table.last_time {
            q = q.bind(v);
        }
        if let Some(v) = &table.gmt_create {
            q = q.bind(v);
        }
        if let Some(v) = &table.gmt_modified {
            q = q.bind(v);
        }
        q = q.bind(name);
        q = q.bind(t);
        q.execute(e).await
    }
}
