use std::time::Duration;

use rbatis::{RBatis, rbdc::DateTime};
use tracing::error;

use crate::storage::tb_lock::TbLock;

pub async fn try_acquire_lock(rb: &RBatis, key: &str, host: &str, ttl_seconds: u64) -> bool {
    let now = DateTime::utc();
    let expires_at = now.clone() + Duration::from_secs(ttl_seconds);

    let sql = r#"
        INSERT INTO tb_lock (`lock_key`, `host`, `expires_at`, `gmt_create`, `gmt_modified`)
        VALUES (?, ?, ?, NOW(), NOW())
        ON DUPLICATE KEY UPDATE
            `host` = IF(tb_lock.expires_at < NOW(), VALUES(`host`), tb_lock.`host`),
            `expires_at` = IF(tb_lock.expires_at < NOW(), VALUES(`expires_at`), tb_lock.`expires_at`),
            `gmt_modified` = IF(tb_lock.expires_at < NOW(), VALUES(`gmt_modified`), tb_lock.`gmt_modified`)
    "#;

    let r = rb
        .exec(sql, vec![key.into(), host.into(), expires_at.into()])
        .await;

    match r {
        Ok(r) => r.rows_affected > 0,
        Err(_) => false,
    }
}

pub async fn release_lock(rb: &RBatis, key: &str) {
    if let Err(e) = TbLock::delete_by_key(rb, key).await {
        error!("Release lock error: {:?}", e);
    }
}
