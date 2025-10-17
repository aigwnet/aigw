use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use aigw_core::{AcmeToken, LogPoint, LogType, Site};
use dashmap::DashMap;
use rusqlite::{Connection, params};
use tokio::sync::Mutex;

pub struct Storage {
    pub(crate) data_dir: PathBuf,
    sqlite_conn: Arc<Mutex<Connection>>,
    sites: arc_swap::ArcSwap<HashMap<String, Arc<Site>>>,
    acme_tokens: arc_swap::ArcSwap<HashMap<String, AcmeToken>>,
    counter: Counter,
}

#[derive(Default)]
pub struct Counter {
    pv: AtomicU64,
    tls: AtomicU64,
    rt: AtomicU64,
    error: AtomicU64,
    http_code_1xx: AtomicU64,
    http_code_2xx: AtomicU64,
    http_code_3xx: AtomicU64,
    http_code_4xx: AtomicU64,
    http_code_5xx: AtomicU64,
    http_source_pc: AtomicU64,
    http_source_mobile: AtomicU64,
    http_source_pad: AtomicU64,
    http_source_bot: AtomicU64,
    http_source_unknown: AtomicU64,
    countries: DashMap<String, AtomicU64>,
}

impl Storage {
    pub fn new(data_dir: Option<&String>) -> anyhow::Result<Self> {
        let path = data_dir.map_or(Err(anyhow::anyhow!("Data directory is null")), |item| {
            Ok(item.parse::<PathBuf>()?)
        })?;
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        let mut db_path = path.clone();
        db_path.push("db");
        if !db_path.exists() {
            fs::create_dir_all(&db_path)?;
        }
        db_path.push("loong.db");

        Ok(Self {
            data_dir: path,
            sqlite_conn: Arc::new(Mutex::new(init_sqlit(&db_path)?)),
            sites: arc_swap::ArcSwap::new(Default::default()),
            acme_tokens: arc_swap::ArcSwap::new(Default::default()),
            counter: Counter::default(),
        })
    }
}

impl Storage {
    pub fn find_site(&self, host: &str) -> Option<Arc<Site>> {
        let servers = self.sites.load();
        servers.get(host).cloned()
    }

    pub fn load_sites(&self) -> anyhow::Result<()> {
        let mut path = self.data_dir.clone();
        path.push("site");
        if !path.exists() {
            return Ok(());
        }
        let files = fs::read_dir(path)?;
        let mut ss = vec![];
        for entry in files {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(path)?;
                let site: Site = serde_json::from_str(&content)?;
                ss.push(Arc::new(site));
            }
        }

        let mut sites = (**self.sites.load()).clone();
        for s in ss {
            for host in &s.alt_names {
                sites.insert(host.to_owned(), s.clone());
            }
            sites.insert(s.name.clone(), s);
        }
        self.sites.store(Arc::new(sites));
        Ok(())
    }

    pub fn add_site(&self, server: Arc<Site>) {
        let mut sites = (**self.sites.load()).clone();
        for host in &server.alt_names {
            sites.insert(host.to_owned(), server.clone());
        }
        sites.insert(server.name.clone(), server);
        self.sites.store(Arc::new(sites));
    }

    pub fn add_token(&self, token: AcmeToken) {
        let mut acme_tokens = (**self.acme_tokens.load()).clone();
        acme_tokens.insert(token.host.clone() + "" + &token.token, token);
        self.acme_tokens.store(Arc::new(acme_tokens));
    }

    pub fn remove_token(&self, host: &str, token: &str) {
        let mut acme_tokens = (**self.acme_tokens.load()).clone();
        let key = host.to_owned() + token;
        acme_tokens.remove(&key);
        self.acme_tokens.store(Arc::new(acme_tokens));
    }

    pub fn find_token(&self, host: &str, token: &str) -> Option<AcmeToken> {
        let key = host.to_owned() + token;
        let acme_tokens = &**self.acme_tokens.load();
        acme_tokens.get(&key).cloned()
    }

    pub async fn update_log_point(&self, log_type: u32, log_id: u64) -> anyhow::Result<()> {
        let conn = self.sqlite_conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO log_point (log_id, log_type) VALUES(?,?)",
            params![log_id, log_type],
        )?;
        Ok(())
    }

    pub async fn load_log_points(&self) -> anyhow::Result<Vec<LogPoint>> {
        let conn = self.sqlite_conn.lock().await;
        let mut stmt = conn.prepare("SELECT log_id, log_type FROM log_point")?;

        let mut res = vec![];
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let log_id = row.get(0)?;
            let log_type: i32 = row.get(1)?;
            if let Ok(log_type) = LogType::try_from(log_type as u32) {
                res.push(LogPoint { log_id, log_type });
            }
        }

        Ok(res)
    }
}

impl Storage {
    pub fn pv(&self) {
        self.counter.pv.fetch_add(1, Ordering::SeqCst);
    }

    pub fn pv_swap(&self) -> u64 {
        self.counter.pv.swap(0, Ordering::SeqCst)
    }

    pub fn tls(&self) {
        self.counter.tls.fetch_add(1, Ordering::SeqCst);
    }

    pub fn tls_swap(&self) -> u64 {
        self.counter.tls.swap(0, Ordering::SeqCst)
    }

    pub fn rt(&self, rt: u64) {
        self.counter.rt.fetch_add(rt, Ordering::SeqCst);
    }

    pub fn rt_swap(&self) -> u64 {
        self.counter.rt.swap(0, Ordering::SeqCst)
    }

    pub fn error(&self) {
        self.counter.error.fetch_add(1, Ordering::SeqCst);
    }

    pub fn error_swap(&self) -> u64 {
        self.counter.error.swap(0, Ordering::SeqCst)
    }

    pub fn http_code_1xx(&self) {
        self.counter.http_code_1xx.fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_code_1xx_swap(&self) -> u64 {
        self.counter.http_code_1xx.swap(0, Ordering::SeqCst)
    }

    pub fn http_code_2xx(&self) {
        self.counter.http_code_2xx.fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_code_2xx_swap(&self) -> u64 {
        self.counter.http_code_2xx.swap(0, Ordering::SeqCst)
    }

    pub fn http_code_3xx(&self) {
        self.counter.http_code_3xx.fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_code_3xx_swap(&self) -> u64 {
        self.counter.http_code_3xx.swap(0, Ordering::SeqCst)
    }

    pub fn http_code_4xx(&self) {
        self.counter.http_code_4xx.fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_code_4xx_swap(&self) -> u64 {
        self.counter.http_code_4xx.swap(0, Ordering::SeqCst)
    }

    pub fn http_code_5xx(&self) {
        self.counter.http_code_5xx.fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_code_5xx_swap(&self) -> u64 {
        self.counter.http_code_5xx.swap(0, Ordering::SeqCst)
    }

    pub fn http_source_pc(&self) {
        self.counter.http_source_pc.fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_source_pc_swap(&self) -> u64 {
        self.counter.http_source_pc.swap(0, Ordering::SeqCst)
    }

    pub fn http_source_mobile(&self) {
        self.counter
            .http_source_mobile
            .fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_source_mobile_swap(&self) -> u64 {
        self.counter.http_source_mobile.swap(0, Ordering::SeqCst)
    }

    pub fn http_source_pad(&self) {
        self.counter.http_source_pad.fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_source_pad_swap(&self) -> u64 {
        self.counter.http_source_pad.swap(0, Ordering::SeqCst)
    }

    pub fn http_source_bot(&self) {
        self.counter.http_source_bot.fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_source_bot_swap(&self) -> u64 {
        self.counter.http_source_bot.swap(0, Ordering::SeqCst)
    }

    pub fn http_source_unknown(&self) {
        self.counter
            .http_source_unknown
            .fetch_add(1, Ordering::SeqCst);
    }

    pub fn http_source_unknown_swap(&self) -> u64 {
        self.counter.http_source_unknown.swap(0, Ordering::SeqCst)
    }

    pub fn country(&self, country: &str) {
        let counter = self
            .counter
            .countries
            .entry(country.to_string())
            .or_insert_with(|| AtomicU64::new(0));

        counter.fetch_add(1, Ordering::SeqCst);
    }

    pub fn countries(&self) -> HashMap<String, u64> {
        let data = self
            .counter
            .countries
            .iter()
            .map(|ref_multi| {
                let country = ref_multi.key().clone();
                let count = ref_multi.value().load(Ordering::SeqCst);
                (country, count)
            })
            .collect();
        self.counter.countries.clear();
        data
    }
}
const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS log_point (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    log_id INTEGER,         
    log_type INTEGER       
);
"#;
const INIT_IDX: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS uniq_idx_type ON log_point(log_type);
"#;
fn init_sqlit(path: &PathBuf) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute(INIT_SQL, ())?;
    conn.execute(INIT_IDX, ())?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::Storage;

    #[test]
    fn test_sqlite() {
        let data_dir = "tmp/data".to_owned();
        let storage = Storage::new(Some(&data_dir)).unwrap();
        let _ = storage.load_log_points();
    }
}
