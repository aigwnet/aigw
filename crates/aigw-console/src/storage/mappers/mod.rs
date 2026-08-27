#[allow(dead_code)]
pub(crate) mod tb_aigw;
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_monitor;
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_monitor_cluster;
#[allow(dead_code)]
pub(crate) mod tb_analytics_monitor_cluster_hour;
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_traffic;
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_traffic_cluster;
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_traffic_cluster_hour;
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_backend;
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_change_log;
#[allow(dead_code)]
pub(crate) mod tb_cluster;
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_cluster_ip_cidr;
#[allow(dead_code)]
pub(crate) mod tb_console;
#[allow(dead_code)]
pub(crate) mod tb_location;
#[allow(dead_code)]
pub(crate) mod tb_lock;
#[allow(dead_code)]
pub(crate) mod tb_session;
#[allow(dead_code)]
pub(crate) mod tb_site;
#[allow(dead_code)]
pub(crate) mod tb_task;
#[allow(dead_code)]
pub(crate) mod tb_user;

pub const DEFAULT_PAGE_SIZE: u64 = 20;

/// Page request, mirrors the old rbatis PageRequest shape.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PageRequest {
    pub page_no: u64,
    pub page_size: u64,
    pub do_count: bool,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            page_no: 1,
            page_size: DEFAULT_PAGE_SIZE,
            do_count: true,
        }
    }
}

impl PageRequest {
    pub fn new(page_no: u64, page_size: u64) -> Self {
        Self {
            page_no: page_no.max(1),
            page_size,
            do_count: true,
        }
    }

    pub fn set_page_size(mut self, arg: u64) -> Self {
        self.page_size = arg;
        self
    }

    pub fn set_page_no(mut self, arg: u64) -> Self {
        self.page_no = arg.max(1);
        self
    }

    pub fn offset(&self) -> u64 {
        (self.page_no - 1) * self.page_size
    }
}

/// Page of records, mirrors the old rbatis Page shape.
#[derive(Debug)]
pub(crate) struct DbPage<T> {
    pub records: Vec<T>,
    pub total: u64,
    pub page_no: u64,
    pub page_size: u64,
}

/// Builds and executes `INSERT INTO <table> (cols...) VALUES (?,...)` skipping
/// `None` fields (DB defaults apply, same as rbatis 4.6 impl_insert!).
/// Option fields go in the first list, always-included fields in the second.
/// Usage: `sqlx_insert!(executor, "tb_user", table, [id, name], [enable]).await`
#[macro_export]
macro_rules! sqlx_insert {
    ($e:expr, $table:expr, $entity:expr, [$($opt:ident),* $(,)?], [$($req:ident),* $(,)?]) => {{
        let mut cols = String::new();
        let mut vals = String::new();
        $(
            if $entity.$opt.is_some() {
                cols.push_str(concat!(stringify!($opt), ","));
                vals.push_str("?,");
            }
        )*
        $(
            cols.push_str(concat!(stringify!($req), ","));
            vals.push_str("?,");
        )*
        cols.pop();
        vals.pop();
        let sql = format!("INSERT INTO {} ({}) VALUES ({})", $table, cols, vals);
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        $(
            if let Some(v) = &$entity.$opt {
                q = q.bind(v);
            }
        )*
        $(
            q = q.bind(&$entity.$req);
        )*
        q.execute($e).await
    }};
}

/// Builds and executes `UPDATE <table> SET <non-null fields> WHERE <where>`;
/// `None` fields are not updated. The last list binds the WHERE params.
/// Usage: `sqlx_update!(executor, "tb_user", table, "name = ?", [name, email], [], [name]).await`
#[macro_export]
macro_rules! sqlx_update {
    ($e:expr, $table:expr, $entity:expr, $where:expr, [$($opt:ident),* $(,)?], [$($req:ident),* $(,)?], [$($wbind:expr),* $(,)?]) => {{
        let mut sets = String::new();
        $(
            if $entity.$opt.is_some() {
                sets.push_str(concat!(stringify!($opt), " = ?,"));
            }
        )*
        $(
            sets.push_str(concat!(stringify!($req), " = ?,"));
        )*
        sets.pop();
        let sql = format!("UPDATE {} SET {} WHERE {}", $table, sets, $where);
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        $(
            if let Some(v) = &$entity.$opt {
                q = q.bind(v);
            }
        )*
        $(
            q = q.bind(&$entity.$req);
        )*
        $(
            q = q.bind($wbind);
        )*
        q.execute($e).await
    }};
}
