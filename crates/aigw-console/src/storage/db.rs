use std::{
    fmt::{Display, Formatter},
    ops::Deref,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use rbatis::{
    DefaultPool, Intercept, RBatis, ResultType, async_trait,
    executor::Executor,
    intercept_page::PageIntercept,
    rbdc::{DateTime, db::ExecResult},
};
use rbdc_mysql::{Driver, options::MySqlConnectOptions};
use rbs::{Error, Value, is_debug_mode};
use tracing::{Level, info, level_filters::LevelFilter};

use crate::storage::tb_user::TbUser;

macro_rules! dynamic_tracing_event {
    ($level:expr, $($field:tt)*) => {
        match $level {
            tracing::Level::ERROR => tracing::error!($($field)*),
            tracing::Level::WARN  => tracing::warn!($($field)*),
            tracing::Level::INFO  => tracing::info!($($field)*),
            tracing::Level::DEBUG => tracing::debug!($($field)*),
            tracing::Level::TRACE => tracing::trace!($($field)*),
        }
    };
}

static INIT_SQL: &str = r#"

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

-- ----------------------------
-- Table structure for tb_analytics_monitor
-- ----------------------------
DROP TABLE IF EXISTS `tb_analytics_monitor`;
CREATE TABLE `tb_analytics_monitor` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `ip` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `uptime` bigint NOT NULL,
  `cpu` double NOT NULL,
  `cpu_current_process` double NOT NULL,
  `cpu_load_one` double NOT NULL,
  `cpu_load_five` double NOT NULL,
  `cpu_load_fifteen` double NOT NULL,
  `mem_used` bigint NOT NULL,
  `mem_free` bigint NOT NULL,
  `swap_used` bigint NOT NULL,
  `swap_free` bigint NOT NULL,
  `disk_used` bigint NOT NULL,
  `disk_free` bigint NOT NULL,
  `io_read` bigint NOT NULL,
  `io_written` bigint NOT NULL,
  `net_send` bigint NOT NULL,
  `net_received` bigint NOT NULL,
  `rt` bigint NOT NULL,
  `error` bigint NOT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_cluster_name_ip` (`cluster_name`,`ip`) USING BTREE,
  KEY `idx_cluster_gmt_create` (`cluster_name`,`gmt_create`),
  KEY `idx_gmt_create` (`gmt_create`),
  CONSTRAINT `tb_analytics_monitor_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_analytics_monitor_cluster
-- ----------------------------
DROP TABLE IF EXISTS `tb_analytics_monitor_cluster`;
CREATE TABLE `tb_analytics_monitor_cluster` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `cpu` double NOT NULL,
  `cpu_current_process` double NOT NULL,
  `cpu_load_one` double NOT NULL,
  `cpu_load_five` double NOT NULL,
  `cpu_load_fifteen` double NOT NULL,
  `mem` double NOT NULL,
  `swap` double NOT NULL,
  `disk` double NOT NULL,
  `io_read` bigint NOT NULL,
  `io_written` bigint NOT NULL,
  `net_send` bigint NOT NULL,
  `net_received` bigint NOT NULL,
  `rt` bigint NOT NULL,
  `error` bigint NOT NULL,
  `gmt_create` timestamp NOT NULL,
  `gmt_modified` timestamp NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_cluster_name_gmt_create` (`cluster_name`,`gmt_create`) USING BTREE,
  CONSTRAINT `tb_analytics_monitor_cluster_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_analytics_monitor_cluster_hour
-- ----------------------------
DROP TABLE IF EXISTS `tb_analytics_monitor_cluster_hour`;
CREATE TABLE `tb_analytics_monitor_cluster_hour` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `cpu` double NOT NULL,
  `cpu_current_process` double NOT NULL,
  `cpu_load_one` double NOT NULL,
  `cpu_load_five` double NOT NULL,
  `cpu_load_fifteen` double NOT NULL,
  `mem` double NOT NULL,
  `swap` double NOT NULL,
  `disk` double NOT NULL,
  `io_read` bigint NOT NULL,
  `io_written` bigint NOT NULL,
  `net_send` bigint NOT NULL,
  `net_received` bigint NOT NULL,
  `rt` bigint NOT NULL,
  `error` bigint NOT NULL,
  `gmt_create` timestamp NOT NULL,
  `gmt_modified` timestamp NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_cluster_name_gmt_create` (`cluster_name`,`gmt_create`) USING BTREE,
  CONSTRAINT `tb_analytics_monitor_cluster_hour_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_analytics_traffic
-- ----------------------------
DROP TABLE IF EXISTS `tb_analytics_traffic`;
CREATE TABLE `tb_analytics_traffic` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `ip` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `pv` bigint NOT NULL,
  `tls` bigint NOT NULL,
  `http_country` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  `http_code` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `http_source` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_cluster_name_gmt_create` (`cluster_name`,`gmt_create`),
  KEY `idx_gmt_create` (`gmt_create`),
  CONSTRAINT `tb_analytics_traffic_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_analytics_traffic_cluster
-- ----------------------------
DROP TABLE IF EXISTS `tb_analytics_traffic_cluster`;
CREATE TABLE `tb_analytics_traffic_cluster` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `pv` bigint NOT NULL,
  `tls` bigint NOT NULL,
  `http_country` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  `http_code` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `http_source` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `gmt_create` timestamp NOT NULL,
  `gmt_modified` timestamp NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_cluster_name_gmt_create` (`cluster_name`,`gmt_create`),
  CONSTRAINT `tb_analytics_traffic_cluster_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_analytics_traffic_cluster_hour
-- ----------------------------
DROP TABLE IF EXISTS `tb_analytics_traffic_cluster_hour`;
CREATE TABLE `tb_analytics_traffic_cluster_hour` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `pv` bigint NOT NULL,
  `tls` bigint NOT NULL,
  `http_country` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  `http_code` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `http_source` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_cluster_name_gmt_create` (`cluster_name`,`gmt_create`),
  KEY `idx_gmt_create` (`gmt_create`),
  CONSTRAINT `tb_analytics_traffic_cluster_hour_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_backend
-- ----------------------------
DROP TABLE IF EXISTS `tb_backend`;
CREATE TABLE `tb_backend` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `location_id` bigint NOT NULL,
  `host` varbinary(255) NOT NULL,
  `port` int NOT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `tb_location_backend_ibfk_1` (`location_id`),
  CONSTRAINT `tb_backend_ibfk_1` FOREIGN KEY (`location_id`) REFERENCES `tb_location` (`id`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_change_log
-- ----------------------------
DROP TABLE IF EXISTS `tb_change_log`;
CREATE TABLE `tb_change_log` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `log_type` int NOT NULL,
  `log_action` int NOT NULL COMMENT '1 add, 2 update, 3 delete',
  `data_id` bigint NOT NULL,
  `data` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  `expire_second` int NOT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_data_id_type` (`data_id`,`log_type`),
  KEY `cluster_name` (`cluster_name`),
  CONSTRAINT `tb_change_log_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_cluster
-- ----------------------------
DROP TABLE IF EXISTS `tb_cluster`;
CREATE TABLE `tb_cluster` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `description` varchar(4096) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_name` (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_console
-- ----------------------------
DROP TABLE IF EXISTS `tb_console`;
CREATE TABLE `tb_console` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `host` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `port` int NOT NULL,
  `last_active_time` timestamp(3) NOT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_host` (`host`,`port`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_location
-- ----------------------------
DROP TABLE IF EXISTS `tb_location`;
CREATE TABLE `tb_location` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `site_id` bigint NOT NULL,
  `location` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `proxy` tinyint NOT NULL COMMENT '1 true, 0 false',
  `protocol` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `sni` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `client_max_body_size` bigint DEFAULT NULL,
  `connection_timeout` int NOT NULL,
  `read_timeout` int NOT NULL,
  `write_timeout` int NOT NULL,
  `idle_timeout` int NOT NULL,
  `rewrite` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `set_headers` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `add_headers` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `root_dir` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `auto_index` tinyint NOT NULL COMMENT '1 true, 0 false',
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_idx_site_loc` (`site_id`,`location`),
  CONSTRAINT `tb_location_ibfk_1` FOREIGN KEY (`site_id`) REFERENCES `tb_site` (`id`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_lock
-- ----------------------------
DROP TABLE IF EXISTS `tb_lock`;
CREATE TABLE `tb_lock` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `lock_key` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `host` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `expires_at` timestamp NOT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_key` (`lock_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_server
-- ----------------------------
DROP TABLE IF EXISTS `tb_server`;
CREATE TABLE `tb_server` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `ip` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `version` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `os_name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `os_version` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `os_arch` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `cpu_name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `cpu_vendor` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `cpu_frequency` bigint NOT NULL,
  `cpu_nums` int NOT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_cluster_name_ip` (`cluster_name`,`ip`) USING BTREE,
  CONSTRAINT `tb_server_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_session
-- ----------------------------
DROP TABLE IF EXISTS `tb_session`;
CREATE TABLE `tb_session` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `user` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `email` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `login_ip` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `token` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_token` (`token`),
  KEY `idx_user_ip` (`user`,`login_ip`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_site
-- ----------------------------
DROP TABLE IF EXISTS `tb_site`;
CREATE TABLE `tb_site` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `alt_names` tinytext COLLATE utf8mb4_unicode_ci,
  `root_dir` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `auto_index` tinyint NOT NULL COMMENT '1 true, 0 false',
  `tls_on` tinyint NOT NULL COMMENT '1 true, 0 false',
  `acme_on` tinyint NOT NULL,
  `tls_cert` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  `tls_cert_start_date` datetime DEFAULT NULL,
  `tls_cert_end_date` datetime DEFAULT NULL,
  `tls_private_key` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_name` (`name`),
  KEY `cluster_name` (`cluster_name`),
  CONSTRAINT `tb_site_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_task
-- ----------------------------
DROP TABLE IF EXISTS `tb_task`;
CREATE TABLE `tb_task` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `type` int NOT NULL,
  `last_time` timestamp(3) NOT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_name_type` (`name`,`type`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_user
-- ----------------------------
DROP TABLE IF EXISTS `tb_user`;
CREATE TABLE `tb_user` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `name` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `email` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `password` varchar(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `real_name` varchar(255) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `ext_info` text COLLATE utf8mb4_unicode_ci,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uniq_name` (`name`),
  UNIQUE KEY `uniq_email` (`email`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

SET FOREIGN_KEY_CHECKS = 1;


"#;

pub(crate) struct DatabaseClient {
    pub(crate) rb: RBatis,
}

impl Default for DatabaseClient {
    fn default() -> Self {
        let rb = Default::default();
        //rb.table_name_filter(move |table_name| format!("my_prefix_{}", table_name));
        Self { rb }
    }
}

impl DatabaseClient {
    /// init mysql with username and password
    pub async fn init(
        &self,
        url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut option = MySqlConnectOptions::from_str(url)?;
        if let Some(password) = password {
            option = option.password(password);
        }
        if let Some(username) = username {
            option = option.username(username);
        }
        self.rb.intercepts.insert(0, Arc::new(PageIntercept::new()));
        self.rb
            .intercepts
            .insert(1, Arc::new(TracingInterceptor::new(LevelFilter::DEBUG)));
        self.rb
            .init_option::<Driver, MySqlConnectOptions, DefaultPool>(Driver {}, option)?;

        self.rb.get_pool()?.set_max_idle_conns(2).await;
        self.rb.get_pool()?.set_max_open_conns(10).await;
        Ok(())
    }

    pub async fn install(&self) -> anyhow::Result<()> {
        self.rb.exec(INIT_SQL, vec![]).await?;
        info!("Create tables successfully.");

        let digest = md5::compute(b"admin");
        let email = "admin@test.test".to_owned();
        let password = format!("{:x}", digest);
        let now = DateTime::utc();

        TbUser::insert(
            &self.rb,
            &TbUser {
                id: None,
                name: Some("admin".to_owned()),
                email: Some(email),
                password: Some(password),
                real_name: Some("Admin".to_string()),
                ext_info: None,
                gmt_create: Some(now.clone()),
                gmt_modified: Some(now),
            },
        )
        .await?;
        info!("Create default user `admin` successfully.");
        info!("Install Completely.");
        Ok(())
    }
}

#[derive(Debug)]
pub struct TracingInterceptor {
    ///control log off,or change log level.
    /// 0=Off,
    /// 1=Error,
    /// 2=Warn,
    /// 3=Info,
    /// 4=Debug,
    /// 5=Trace
    pub level_filter: AtomicUsize,
}

impl TracingInterceptor {
    pub fn new(level_filter: LevelFilter) -> Self {
        let s = Self {
            level_filter: AtomicUsize::new(0),
        };
        s.set_level_filter(level_filter);
        s
    }

    pub fn get_level_filter(&self) -> LevelFilter {
        match self.level_filter.load(Ordering::Relaxed) {
            0 => LevelFilter::OFF,
            1 => LevelFilter::ERROR,
            2 => LevelFilter::WARN,
            3 => LevelFilter::INFO,
            4 => LevelFilter::DEBUG,
            5 => LevelFilter::TRACE,
            _ => LevelFilter::OFF,
        }
    }

    pub fn to_level(&self) -> Option<Level> {
        match self.get_level_filter() {
            LevelFilter::OFF => None,
            LevelFilter::ERROR => Some(Level::ERROR),
            LevelFilter::WARN => Some(Level::WARN),
            LevelFilter::INFO => Some(Level::INFO),
            LevelFilter::DEBUG => Some(Level::DEBUG),
            LevelFilter::TRACE => Some(Level::TRACE),
        }
    }

    pub fn set_level_filter(&self, level_filter: LevelFilter) {
        match level_filter {
            LevelFilter::OFF => self.level_filter.store(0, Ordering::SeqCst),
            LevelFilter::ERROR => self.level_filter.store(1, Ordering::SeqCst),
            LevelFilter::WARN => self.level_filter.store(2, Ordering::SeqCst),
            LevelFilter::INFO => self.level_filter.store(3, Ordering::SeqCst),
            LevelFilter::DEBUG => self.level_filter.store(4, Ordering::SeqCst),
            LevelFilter::TRACE => self.level_filter.store(5, Ordering::SeqCst),
        }
    }
}

#[async_trait]
impl Intercept for TracingInterceptor {
    async fn before(
        &self,
        task_id: i64,
        _rb: &dyn Executor,
        sql: &mut String,
        args: &mut Vec<Value>,
        _result: ResultType<&mut Result<ExecResult, Error>, &mut Result<Vec<Value>, Error>>,
    ) -> Result<Option<bool>, Error> {
        if self.get_level_filter() == LevelFilter::OFF {
            return Ok(Some(true));
        }
        let level = self.to_level().unwrap_or(Level::DEBUG);
        //send sql/args
        dynamic_tracing_event!(
            level, target: "database",
            "[rb] [{}] => `{}` {}",
            task_id,
            &sql,
            RbsValueDisplay::new(args)
        );

        Ok(Some(true))
    }

    async fn after(
        &self,
        task_id: i64,
        _rb: &dyn Executor,
        _sql: &mut String,
        _args: &mut Vec<Value>,
        result: ResultType<&mut Result<ExecResult, Error>, &mut Result<Vec<Value>, Error>>,
    ) -> Result<Option<bool>, Error> {
        if self.get_level_filter() == LevelFilter::OFF {
            return Ok(Some(true));
        }
        let level = self.to_level().unwrap_or_else(|| Level::DEBUG);
        //ResultType
        match result {
            ResultType::Exec(result) => match result {
                Ok(result) => {
                    dynamic_tracing_event!(level, target: "database", "[rb] [{}] <= rows_affected={}", task_id, result);
                }
                Err(e) => {
                    dynamic_tracing_event!(level, target: "database", "[rb] [{}] <= {}", task_id, e);
                }
            },
            ResultType::Query(result) => match result {
                Ok(result) => {
                    if is_debug_mode() {
                        dynamic_tracing_event!(
                            level, target: "database",
                            "[rb] [{}] <= len={},rows={}",
                            task_id,
                            result.len(),
                            RbsValueDisplay { inner: result }
                        );
                    } else {
                        dynamic_tracing_event!(level, target: "database", "[rb] [{}] <= len={}", task_id, result.len());
                    }
                }
                Err(e) => {
                    dynamic_tracing_event!(level, target: "database", "[rb] [{}] <= {}", task_id, e);
                }
            },
        }
        Ok(Some(true))
    }
}

struct RbsValueDisplay<'a> {
    inner: &'a Vec<Value>,
}

impl<'a> RbsValueDisplay<'a> {
    pub fn new(v: &'a Vec<Value>) -> Self {
        Self { inner: v }
    }
}

impl<'a> Display for RbsValueDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        let mut idx = 0;
        for x in self.inner.deref() {
            std::fmt::Display::fmt(x, f)?;
            if (idx + 1) < self.inner.len() {
                f.write_str(",")?;
            }
            idx += 1;
        }
        f.write_str("]")?;
        Ok(())
    }
}
