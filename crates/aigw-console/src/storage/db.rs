use std::{str::FromStr, sync::Arc};

use log::{LevelFilter, info};
use rbatis::{
    DefaultPool, RBatis, intercept_log::LogInterceptor, intercept_page::PageIntercept,
    rbdc::DateTime,
};
use rbdc_mysql::{Driver, options::MySqlConnectOptions};

use crate::storage::tb_user::TbUser;

static INIT_SQL: &str = r#"

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

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
-- Table structure for tb_dinosaur
-- ----------------------------
DROP TABLE IF EXISTS `tb_dinosaur`;
CREATE TABLE `tb_dinosaur` (
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
            .insert(1, Arc::new(LogInterceptor::new(LevelFilter::Debug)));
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
