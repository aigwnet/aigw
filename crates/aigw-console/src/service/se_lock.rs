use std::time::Duration;

use time::OffsetDateTime;
use tracing::error;

use crate::storage::tb_lock::TbLock;

pub async fn try_acquire_lock(rb: &sqlx::MySqlPool, key: &str, host: &str, ttl_seconds: u64) -> bool {
    let now = OffsetDateTime::now_utc();
    let expires_at = now + Duration::from_secs(ttl_seconds);

    let sql = r#"
        INSERT INTO tb_lock (`lock_key`, `host`, `expires_at`, `gmt_create`, `gmt_modified`)
        VALUES (?, ?, ?, NOW(), NOW())
        ON DUPLICATE KEY UPDATE
            `host` = IF(tb_lock.expires_at < NOW(), VALUES(`host`), tb_lock.`host`),
            `expires_at` = IF(tb_lock.expires_at < NOW(), VALUES(`expires_at`), tb_lock.`expires_at`),
            `gmt_modified` = IF(tb_lock.expires_at < NOW(), VALUES(`gmt_modified`), tb_lock.`gmt_modified`)
    "#;

    let r = sqlx::query(sql)
        .bind(key)
        .bind(host)
        .bind(expires_at)
        .execute(rb)
        .await;

    match r {
        Ok(r) => r.rows_affected() > 0,
        Err(_) => false,
    }
}

pub async fn release_lock(rb: &sqlx::MySqlPool, key: &str) {
    if let Err(e) = TbLock::delete_by_key(rb, key).await {
        error!("Release lock error: {:?}", e);
    }
}
