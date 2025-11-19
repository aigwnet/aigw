use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use aigw_core::{AcmeToken, Cluster, LogPoint, LogType, Site};
use dashmap::DashMap;
use pingora_limits::rate::Rate;
use rusqlite::{Connection, params};
use rustls::{
    crypto::{CryptoProvider, aws_lc_rs::default_provider},
    sign::CertifiedKey,
};
use tokio::sync::Mutex;

use crate::server::RateLimit;

pub struct Storage {
    pub(crate) data_dir: PathBuf,
    cluster: String,
    sqlite_conn: Arc<Mutex<Connection>>,
    cluster_config: arc_swap::ArcSwap<Cluster>,
    sites: arc_swap::ArcSwap<HashMap<String, Arc<Site>>>,
    rates: arc_swap::ArcSwap<HashMap<String, Arc<RateLimit>>>,
    default_tls_site: arc_swap::ArcSwap<Option<Arc<Site>>>,
    acme_tokens: arc_swap::ArcSwap<HashMap<String, AcmeToken>>,
    counter: Counter,
    crypto_provider: CryptoProvider,
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
    pub fn new(data_dir: Option<&String>, cluster: String) -> anyhow::Result<Self> {
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
        db_path.push("aigw.db");

        let provider = default_provider();

        Ok(Self {
            data_dir: path,
            cluster: cluster.clone(),
            cluster_config: arc_swap::ArcSwap::new(Arc::new(Cluster {
                id: None,
                name: cluster,
                security_key: "".to_string(),
                enable: false,
                enable_default_site: false,
                enable_white_list: false,
                enable_block_list: false,
                description: None,
                gmt_modified: None,
            })),
            sqlite_conn: Arc::new(Mutex::new(init_sqlit(&db_path)?)),
            sites: arc_swap::ArcSwap::new(Default::default()),
            rates: arc_swap::ArcSwap::new(Default::default()),
            default_tls_site: arc_swap::ArcSwap::new(Default::default()),
            acme_tokens: arc_swap::ArcSwap::new(Default::default()),
            counter: Counter::default(),
            crypto_provider: provider,
        })
    }
}

impl Storage {
    pub fn find_site(&self, host: &str) -> Option<Arc<Site>> {
        let sites = self.sites.load();
        sites.get(host).cloned()
    }

    pub fn find_rate(&self, host: &str) -> Option<Arc<RateLimit>> {
        let rates = self.rates.load();
        rates.get(host).cloned()
    }

    pub fn find_default_tls_site(&self) -> Option<Arc<Site>> {
        if self.cluster().enable_default_site {
            (**self.default_tls_site.load()).clone()
        } else {
            None
        }
    }

    pub fn load_cluster(&self) -> anyhow::Result<()> {
        let mut path = self.data_dir.clone();
        if !path.exists() {
            return Ok(());
        }
        path.push("cluster.json");

        if let Ok(content) = fs::read_to_string(&path) {
            let cluster: Cluster = serde_json::from_str(&content)?;
            self.store_cluster(Arc::new(cluster));
        }

        Ok(())
    }

    pub fn cluster(&self) -> Arc<Cluster> {
        self.cluster_config.load().clone()
    }

    pub fn store_cluster(&self, cluster: Arc<Cluster>) {
        self.cluster_config.store(cluster);
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
                let content = fs::read_to_string(&path)?;
                let mut site: Site = serde_json::from_str(&content)?;
                if !site.cluster.eq(&self.cluster) {
                    // Delete invalid data
                    let _ = fs::remove_file(&path);
                    continue;
                }

                self.fill_certified_key(&mut site)?;
                ss.push(Arc::new(site));
            }
        }

        let mut sites = HashMap::new();
        let mut rates = HashMap::new();
        for s in ss {
            let rate_limit = Arc::new(RateLimit {
                max_request: s.rate_limit,
                rate: Rate::new(Duration::from_millis(s.rate_limit_unit)),
            });
            for host in &s.alt_names {
                sites.insert(host.to_owned(), s.clone());
                if s.rate_limit > 0 {
                    rates.insert(host.to_owned(), rate_limit.clone());
                }
            }

            // set default site
            if self.default_tls_site.load().is_none() && s.tls_on {
                self.default_tls_site.store(Arc::new(Some(s.clone())));
            }
            if s.rate_limit > 0 {
                rates.insert(s.name.clone(), rate_limit.clone());
            }
            sites.insert(s.name.clone(), s);
        }
        self.sites.store(Arc::new(sites));
        self.rates.store(Arc::new(rates));
        Ok(())
    }

    pub fn add_site(&self, mut site: Site) -> anyhow::Result<()> {
        self.fill_certified_key(&mut site)?;
        let site = Arc::new(site);
        let mut sites = (**self.sites.load()).clone();
        let mut rates = (**self.rates.load()).clone();

        let rate_limit = Arc::new(RateLimit {
            max_request: site.rate_limit,
            rate: Rate::new(Duration::from_millis(site.rate_limit_unit)),
        });
        for host in &site.alt_names {
            sites.insert(host.to_owned(), site.clone());
            if site.rate_limit > 0 {
                rates.insert(host.to_owned(), rate_limit.clone());
            }
        }

        if self.default_tls_site.load().is_none() && site.tls_on {
            self.default_tls_site.store(Arc::new(Some(site.clone())));
        }
        if site.rate_limit > 0 {
            rates.insert(site.name.clone(), rate_limit.clone());
        }
        sites.insert(site.name.clone(), site);

        self.sites.store(Arc::new(sites));
        self.rates.store(Arc::new(rates));

        Ok(())
    }

    pub fn remove_site(&self, site: &Site) {
        let mut sites = (**self.sites.load()).clone();
        let mut rates = (**self.rates.load()).clone();
        for host in &site.alt_names {
            sites.remove(host);
            rates.remove(host);
        }
        sites.remove(&site.name);
        rates.remove(&site.name);

        if let Some(default_site) = &**self.default_tls_site.load()
            && default_site.name.eq(&site.name)
        {
            for site in sites.values() {
                if site.tls_on {
                    self.default_tls_site.store(Arc::new(Some(site.clone())));
                    break;
                }
            }
        }
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

    fn fill_certified_key(&self, site: &mut Site) -> anyhow::Result<()> {
        if let Some(key) = &site.tls_private_key {
            if let Some(c) = &site.tls_cert {
                let mut certs = vec![c.cert.cert().clone()];
                for c in &c.cert_chain {
                    certs.push(c.cert().clone());
                }
                let private_key = self
                    .crypto_provider
                    .key_provider
                    .load_private_key(key.0.clone_key())?;
                site.certified_key = Some(Arc::new(CertifiedKey::new(certs, private_key)));
            }
        }
        Ok(())
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
        let storage = Storage::new(Some(&data_dir), "test".to_string()).unwrap();
        let _ = storage.load_log_points();
    }
}
