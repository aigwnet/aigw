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
    acme_tokens: arc_swap::ArcSwap<HashMap<String, (AcmeToken, std::time::Instant)>>,
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
                real_ip_from: vec![],
                description: None,
                gmt_modified: None,
            })),
            sqlite_conn: Arc::new(Mutex::new(init_sqlite(&db_path)?)),
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
    /// Lowercase hostnames are used as map keys; DNS names are case-insensitive.
    fn host_key(host: &str) -> String {
        host.to_lowercase()
    }

    pub fn find_site(&self, host: &str) -> Option<Arc<Site>> {
        let sites = self.sites.load();
        if host.bytes().any(|b| b.is_ascii_uppercase()) {
            sites.get(&Self::host_key(host)).cloned()
        } else {
            sites.get(host).cloned()
        }
    }

    pub fn find_rate(&self, host: &str) -> Option<Arc<RateLimit>> {
        let rates = self.rates.load();
        if host.bytes().any(|b| b.is_ascii_uppercase()) {
            rates.get(&Self::host_key(host)).cloned()
        } else {
            rates.get(host).cloned()
        }
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
                sites.insert(Self::host_key(host), s.clone());
                if s.rate_limit > 0 {
                    rates.insert(Self::host_key(host), rate_limit.clone());
                }
            }

            // set default site
            if self.default_tls_site.load().is_none() && s.tls_on {
                self.default_tls_site.store(Arc::new(Some(s.clone())));
            }
            if s.rate_limit > 0 {
                rates.insert(Self::host_key(&s.name), rate_limit.clone());
            }
            sites.insert(Self::host_key(&s.name), s);
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

        // On update, drop stale hostname mappings from the previous version of
        // this site, but only if they still point to this site.
        let site_key = Self::host_key(&site.name);
        let new_alt_keys: Vec<String> = site.alt_names.iter().map(|h| Self::host_key(h)).collect();
        if let Some(old) = sites.get(&site_key).cloned() {
            for host in &old.alt_names {
                let key = Self::host_key(host);
                if !new_alt_keys.contains(&key)
                    && sites.get(&key).is_some_and(|s| s.name == site.name)
                {
                    sites.remove(&key);
                    rates.remove(&key);
                }
            }
        }

        let rate_limit = Arc::new(RateLimit {
            max_request: site.rate_limit,
            rate: Rate::new(Duration::from_millis(site.rate_limit_unit)),
        });
        for host in &site.alt_names {
            let key = Self::host_key(host);
            sites.insert(key.clone(), site.clone());
            if site.rate_limit > 0 {
                rates.insert(key.clone(), rate_limit.clone());
            } else {
                // rate limiting disabled by this update: drop stale entries
                rates.remove(&key);
            }
        }

        if site.rate_limit > 0 {
            rates.insert(site_key.clone(), rate_limit.clone());
        } else {
            rates.remove(&site_key);
        }
        sites.insert(site_key.clone(), site.clone());

        // Keep the default TLS site fresh: pick this site when there is no
        // default yet, when this update replaces the current default, or when
        // the current default no longer exists / lost TLS.
        let default = (**self.default_tls_site.load()).clone();
        let refresh_default = match &default {
            None => site.tls_on,
            Some(d) => {
                d.name == site.name || !d.tls_on || !sites.contains_key(&Self::host_key(&d.name))
            }
        };
        if refresh_default {
            if site.tls_on {
                self.default_tls_site.store(Arc::new(Some(site)));
            } else {
                let replacement = sites.values().find(|s| s.tls_on).cloned();
                self.default_tls_site.store(Arc::new(replacement));
            }
        }

        self.sites.store(Arc::new(sites));
        self.rates.store(Arc::new(rates));

        Ok(())
    }

    pub fn remove_site(&self, site: &Site) {
        let mut sites = (**self.sites.load()).clone();
        let mut rates = (**self.rates.load()).clone();
        // Only remove hostname mappings that still point to this site; another
        // site may have claimed the same hostname afterwards.
        for host in &site.alt_names {
            let key = Self::host_key(host);
            if sites.get(&key).is_some_and(|s| s.name == site.name) {
                sites.remove(&key);
                rates.remove(&key);
            }
        }
        let site_key = Self::host_key(&site.name);
        sites.remove(&site_key);
        rates.remove(&site_key);

        if let Some(default_site) = &**self.default_tls_site.load()
            && default_site.name.eq(&site.name)
        {
            // Pick another TLS site as default, or clear it entirely so the
            // deleted site (and its cert) is no longer served.
            let replacement = sites.values().find(|s| s.tls_on).cloned();
            self.default_tls_site.store(Arc::new(replacement));
        }
        self.sites.store(Arc::new(sites));
        self.rates.store(Arc::new(rates));
    }

    /// ACME http-01 tokens are short-lived; the console expires them after 300s
    const ACME_TOKEN_TTL: std::time::Duration = std::time::Duration::from_secs(300);

    /// Separator keeps (host, token) pairs unambiguous (e.g. "ab"+"c" vs "a"+"bc")
    fn acme_key(host: &str, token: &str) -> String {
        format!("{host}\n{token}")
    }

    pub fn add_token(&self, token: AcmeToken) {
        let mut acme_tokens = (**self.acme_tokens.load()).clone();
        acme_tokens.insert(
            Self::acme_key(&token.host, &token.token),
            (token, std::time::Instant::now()),
        );
        self.acme_tokens.store(Arc::new(acme_tokens));
    }

    pub fn remove_token(&self, host: &str, token: &str) {
        let mut acme_tokens = (**self.acme_tokens.load()).clone();
        acme_tokens.remove(&Self::acme_key(host, token));
        self.acme_tokens.store(Arc::new(acme_tokens));
    }

    pub fn find_token(&self, host: &str, token: &str) -> Option<AcmeToken> {
        let key = Self::acme_key(host, token);
        let acme_tokens = &**self.acme_tokens.load();
        acme_tokens
            .get(&key)
            .filter(|(_, ts)| ts.elapsed() < Self::ACME_TOKEN_TTL)
            .map(|(token, _)| token)
            .cloned()
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

    #[cfg(target_os = "linux")]
    pub async fn add_ip_cidr(&self, list: &aigw_core::IpList) -> anyhow::Result<()> {
        let conn = self.sqlite_conn.lock().await;
        for ip in &list.data {
            conn.execute(
                "INSERT INTO cluster_ip_cidr (ip, prefix_len, type) VALUES(?,?,?)",
                params![ip.data, ip.prefix_len, list.item_type],
            )?;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub async fn remove_ip_cidr(&self, list: &aigw_core::IpList) -> anyhow::Result<()> {
        let conn = self.sqlite_conn.lock().await;
        for ip in &list.data {
            conn.execute(
                "DELETE FROM cluster_ip_cidr WHERE ip=? AND prefix_len=? AND type=?",
                params![ip.data, ip.prefix_len, list.item_type],
            )?;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub async fn load_ip_cidr(&self, r#type: i32) -> anyhow::Result<Vec<aigw_core::IpItem>> {
        let conn = self.sqlite_conn.lock().await;

        let mut stmt = conn.prepare("SELECT ip, prefix_len FROM cluster_ip_cidr WHERE type=?")?;

        let mut res = vec![];
        let mut rows = stmt.query(params![r#type])?;
        while let Some(row) = rows.next()? {
            let ip = row.get(0)?;
            let prefix_len: u32 = row.get(1)?;
            res.push(aigw_core::IpItem {
                data: ip,
                prefix_len,
            });
        }
        Ok(res)
    }

    fn fill_certified_key(&self, site: &mut Site) -> anyhow::Result<()> {
        if let Some(key) = &site.tls_private_key
            && let Some(c) = &site.tls_cert
        {
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
const INIT_SQL_LOG_POINT: &str = r#"
CREATE TABLE IF NOT EXISTS log_point (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    log_id INTEGER,         
    log_type INTEGER       
);
"#;
const INIT_IDX_LOG_POINT: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS uniq_idx_type ON log_point(log_type);
"#;

#[cfg(target_os = "linux")]
const INIT_SQL_IP_CIDR: &str = r#"
CREATE TABLE IF NOT EXISTS cluster_ip_cidr (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ip TEXT,
    prefix_len INTEGER,
    type INTEGER
);
"#;
#[cfg(target_os = "linux")]
const INIT_IDX_IP_CIDR: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS uniq_idx_type_ip ON cluster_ip_cidr(type, ip, prefix_len);
"#;

fn init_sqlite(path: &PathBuf) -> anyhow::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute(INIT_SQL_LOG_POINT, ())?;
    conn.execute(INIT_IDX_LOG_POINT, ())?;
    #[cfg(target_os = "linux")]
    {
        conn.execute(INIT_SQL_IP_CIDR, ())?;
        conn.execute(INIT_IDX_IP_CIDR, ())?;
    }

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::Storage;
    use aigw_core::Site;

    #[test]
    fn test_sqlite() {
        let data_dir = "tmp/data".to_owned();
        let storage = Storage::new(Some(&data_dir), "test".to_string()).unwrap();
        let _ = storage.load_log_points();
    }

    fn make_site(name: &str, alt_names: &[&str], tls_on: bool) -> Site {
        serde_json::from_value(serde_json::json!({
            "cluster": "test",
            "name": name,
            "alt_names": alt_names,
            "auto_index": false,
            "tls_on": tls_on,
            "rate_limit": 10,
            "rate_limit_unit": 1000u64,
            "locations": []
        }))
        .unwrap()
    }

    fn default_tls_site_name(storage: &Storage) -> Option<String> {
        storage
            .default_tls_site
            .load()
            .as_ref()
            .as_ref()
            .map(|s| s.name.clone())
    }

    #[test]
    fn test_site_add_update_remove() {
        let data_dir = format!("tmp/data-site-test-{}", std::process::id());
        let storage = Storage::new(Some(&data_dir), "test".to_string()).unwrap();

        // Add a TLS site: becomes the default TLS site
        storage
            .add_site(make_site("a.com", &["www.a.com"], true))
            .unwrap();
        assert!(storage.find_site("www.a.com").is_some());
        assert!(storage.find_rate("a.com").is_some());
        assert_eq!(default_tls_site_name(&storage).as_deref(), Some("a.com"));

        // Update: drop www.a.com, add m.a.com, disable rate limiting
        let mut updated = make_site("a.com", &["m.a.com"], true);
        updated.rate_limit = 0;
        storage.add_site(updated).unwrap();
        assert!(storage.find_site("www.a.com").is_none());
        assert!(storage.find_site("m.a.com").is_some());
        assert!(storage.find_rate("a.com").is_none());
        assert_eq!(default_tls_site_name(&storage).as_deref(), Some("a.com"));

        // Remove the site: rate entries cleaned, default cleared (no other TLS site)
        storage.remove_site(&make_site("a.com", &["m.a.com"], true));
        assert!(storage.find_site("a.com").is_none());
        assert!(storage.find_site("m.a.com").is_none());
        assert!(storage.find_rate("a.com").is_none());
        assert!(default_tls_site_name(&storage).is_none());

        // Two TLS sites: removing the default falls back to the other one
        storage.add_site(make_site("a.com", &[], true)).unwrap();
        storage.add_site(make_site("b.com", &[], true)).unwrap();
        storage.remove_site(&make_site("a.com", &[], true));
        assert_eq!(default_tls_site_name(&storage).as_deref(), Some("b.com"));

        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
