use sqlx::MySqlExecutor;
use sqlx::mysql::MySqlQueryResult;
use time::OffsetDateTime;

use super::{DbPage, PageRequest};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TbClusterIpCidr {
    pub id: Option<i64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub prefix_len: Option<i32>,
    pub r#type: Option<i8>,
    pub start_time: Option<OffsetDateTime>,
    pub end_time: Option<OffsetDateTime>,
    pub gmt_create: Option<OffsetDateTime>,
    pub gmt_modified: Option<OffsetDateTime>,
}

impl TbClusterIpCidr {
    /// insert: None fields are skipped (DB defaults apply).
    /// Hand-written instead of sqlx_insert! because stringify!(r#type) yields
    /// "r#type" instead of the column name `type`; semantics are identical.
    pub async fn insert<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbClusterIpCidr,
    ) -> sqlx::Result<MySqlQueryResult> {
        let mut cols = String::new();
        let mut vals = String::new();
        if table.id.is_some() {
            cols.push_str("id,");
            vals.push_str("?,");
        }
        if table.cluster_name.is_some() {
            cols.push_str("cluster_name,");
            vals.push_str("?,");
        }
        if table.ip.is_some() {
            cols.push_str("ip,");
            vals.push_str("?,");
        }
        if table.prefix_len.is_some() {
            cols.push_str("prefix_len,");
            vals.push_str("?,");
        }
        if table.r#type.is_some() {
            cols.push_str("`type`,");
            vals.push_str("?,");
        }
        if table.start_time.is_some() {
            cols.push_str("start_time,");
            vals.push_str("?,");
        }
        if table.end_time.is_some() {
            cols.push_str("end_time,");
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
        let sql = format!("INSERT INTO tb_cluster_ip_cidr ({}) VALUES ({})", cols, vals);
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        if let Some(v) = &table.id {
            q = q.bind(v);
        }
        if let Some(v) = &table.cluster_name {
            q = q.bind(v);
        }
        if let Some(v) = &table.ip {
            q = q.bind(v);
        }
        if let Some(v) = &table.prefix_len {
            q = q.bind(v);
        }
        if let Some(v) = &table.r#type {
            q = q.bind(v);
        }
        if let Some(v) = &table.start_time {
            q = q.bind(v);
        }
        if let Some(v) = &table.end_time {
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

    pub async fn select_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        id: i64,
    ) -> sqlx::Result<Option<TbClusterIpCidr>> {
        sqlx::query_as::<_, TbClusterIpCidr>("SELECT * FROM tb_cluster_ip_cidr WHERE id = ?")
            .bind(id)
            .fetch_optional(e)
            .await
    }

    pub async fn delete_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query("DELETE FROM tb_cluster_ip_cidr WHERE id = ?")
            .bind(id)
            .execute(e)
            .await
    }

    /// Page query (read-only, takes the pool directly so it can run two statements)
    pub async fn select_page(
        pool: &sqlx::MySqlPool,
        page_request: &PageRequest,
        cluster_name: &str,
        t: i8,
    ) -> sqlx::Result<DbPage<TbClusterIpCidr>> {
        let mut page = DbPage {
            records: vec![],
            total: 0,
            page_no: page_request.page_no,
            page_size: page_request.page_size,
        };
        if page_request.do_count {
            let (total,): (i64,) = sqlx::query_as(
                "SELECT count(1) FROM tb_cluster_ip_cidr WHERE cluster_name = ? and `type` = ?",
            )
            .bind(cluster_name)
            .bind(t)
            .fetch_one(pool)
            .await?;
            page.total = total as u64;
        }
        page.records = sqlx::query_as::<_, TbClusterIpCidr>(
            "SELECT * FROM tb_cluster_ip_cidr WHERE cluster_name = ? and `type` = ? ORDER BY id DESC LIMIT ?,?",
        )
        .bind(cluster_name)
        .bind(t)
        .bind(page_request.offset())
        .bind(page_request.page_size)
        .fetch_all(pool)
        .await?;
        Ok(page)
    }

    /// update: None fields are not updated.
    /// Hand-written instead of sqlx_update! because stringify!(r#type) yields
    /// "r#type" instead of the column name `type`; semantics are identical.
    pub async fn update_by_id<'e, E: MySqlExecutor<'e>>(
        e: E,
        table: &TbClusterIpCidr,
        id: i64,
    ) -> sqlx::Result<MySqlQueryResult> {
        let mut sets = String::new();
        if table.cluster_name.is_some() {
            sets.push_str("cluster_name = ?,");
        }
        if table.ip.is_some() {
            sets.push_str("ip = ?,");
        }
        if table.prefix_len.is_some() {
            sets.push_str("prefix_len = ?,");
        }
        if table.r#type.is_some() {
            sets.push_str("`type` = ?,");
        }
        if table.start_time.is_some() {
            sets.push_str("start_time = ?,");
        }
        if table.end_time.is_some() {
            sets.push_str("end_time = ?,");
        }
        if table.gmt_create.is_some() {
            sets.push_str("gmt_create = ?,");
        }
        if table.gmt_modified.is_some() {
            sets.push_str("gmt_modified = ?,");
        }
        sets.pop();
        let sql = format!("UPDATE tb_cluster_ip_cidr SET {} WHERE id = ?", sets);
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        if let Some(v) = &table.cluster_name {
            q = q.bind(v);
        }
        if let Some(v) = &table.ip {
            q = q.bind(v);
        }
        if let Some(v) = &table.prefix_len {
            q = q.bind(v);
        }
        if let Some(v) = &table.r#type {
            q = q.bind(v);
        }
        if let Some(v) = &table.start_time {
            q = q.bind(v);
        }
        if let Some(v) = &table.end_time {
            q = q.bind(v);
        }
        if let Some(v) = &table.gmt_create {
            q = q.bind(v);
        }
        if let Some(v) = &table.gmt_modified {
            q = q.bind(v);
        }
        q = q.bind(id);
        q.execute(e).await
    }
}
