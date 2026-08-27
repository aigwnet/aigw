use std::str::FromStr;

use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};
use time::OffsetDateTime;
use tracing::info;

use crate::storage::tb_user::TbUser;

static INIT_SQL: &str = r#"


SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

-- ----------------------------
-- Table structure for tb_aigw
-- ----------------------------
DROP TABLE IF EXISTS `tb_aigw`;
CREATE TABLE `tb_aigw` (
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
  CONSTRAINT `tb_aigw_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

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
  `host` varchar(255) NOT NULL,
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
  `security_key` varchar(4096) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
  `enable` tinyint NOT NULL,
  `enable_default_site` tinyint NOT NULL,
  `enable_white_list` tinyint NOT NULL,
  `enable_block_list` tinyint NOT NULL,
  `description` varchar(4096) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `idx_name` (`name`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------
-- Table structure for tb_cluster_ip_cidr
-- ----------------------------
DROP TABLE IF EXISTS `tb_cluster_ip_cidr`;
CREATE TABLE `tb_cluster_ip_cidr` (
  `id` bigint NOT NULL AUTO_INCREMENT,
  `cluster_name` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `ip` varchar(255) COLLATE utf8mb4_unicode_ci NOT NULL,
  `prefix_len` int NOT NULL,
  `type` tinyint NOT NULL,
  `start_time` timestamp(3) NULL DEFAULT NULL,
  `end_time` timestamp(3) NULL DEFAULT NULL,
  `gmt_create` timestamp(3) NOT NULL,
  `gmt_modified` timestamp(3) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `cluster_name` (`cluster_name`),
  CONSTRAINT `tb_cluster_ip_cidr_ibfk_1` FOREIGN KEY (`cluster_name`) REFERENCES `tb_cluster` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
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
  `http_version` varchar(4) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `proxy_set_headers` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `proxy_add_headers` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `proxy_remove_headers` varchar(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `response_set_headers` varchar(1024) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `response_add_headers` varchar(1024) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
  `response_remove_headers` varchar(1024) COLLATE utf8mb4_unicode_ci DEFAULT NULL,
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
  `tls_enforce` tinyint NOT NULL,
  `acme_on` tinyint NOT NULL,
  `tls_cert` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  `tls_cert_start_date` datetime DEFAULT NULL,
  `tls_cert_end_date` datetime DEFAULT NULL,
  `tls_private_key` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  `rate_limit` bigint NOT NULL,
  `rate_limit_unit` bigint NOT NULL,
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
    pub(crate) rb: MySqlPool,
}

impl DatabaseClient {
    /// init mysql pool with username and password
    pub async fn init(
        url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> anyhow::Result<Self> {
        let mut option = MySqlConnectOptions::from_str(url)?;
        if let Some(password) = password {
            option = option.password(password);
        }
        if let Some(username) = username {
            option = option.username(username);
        }
        let rb = MySqlPoolOptions::new()
            .max_connections(10)
            .connect_with(option)
            .await?;
        Ok(Self { rb })
    }

    pub async fn install(&self) -> anyhow::Result<()> {
        sqlx::raw_sql(INIT_SQL).execute(&self.rb).await?;
        info!("Create tables successfully.");

        let digest = md5::compute(b"admin");
        let email = "admin@test.test".to_owned();
        let password = format!("{:x}", digest);
        let now = OffsetDateTime::now_utc();

        TbUser::insert(
            &self.rb,
            &TbUser {
                id: None,
                name: Some("admin".to_owned()),
                email: Some(email),
                password: Some(password),
                real_name: Some("Admin".to_string()),
                ext_info: None,
                gmt_create: Some(now),
                gmt_modified: Some(now),
            },
        )
        .await?;
        info!("Create default user `admin` successfully.");
        info!("Install Completely.");
        Ok(())
    }
}
