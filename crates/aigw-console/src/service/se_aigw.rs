use aigw_core::{HandshakeInfo, date_format_local};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    service::{Page, YYYY_MM_DD_HH_MM_SS_FORMAT},
    storage::{PageRequest, tb_aigw::TbAigw},
};

/// Asynchronously updates an existing AIGW or inserts a new one based on the handshake information.
///
/// # Parameters
/// - `rb`: Reference to the RBatis instance for database operations
/// - `info`: HandshakeInfo containing the AIGW data to be stored or updated
///
/// # Returns
/// - `Ok(())` on successful update/insert operation
/// - `Err(anyhow::Error)` if database operations fail, serialization errors occur, or constraints are violated
///
/// # Errors
/// Returns an error if database operations fail or if required parameters are invalid.
pub async fn update_or_insert_aigw(
    rb: &sqlx::MySqlPool,
    info: HandshakeInfo,
) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc();
    let item = TbAigw::select_by_cluster_name_and_ip(rb, &info.cluster, &info.ip).await?;
    // update last_active_time
    if let Some(mut item) = item {
        item.version = Some(info.version);
        item.os_name = Some(info.os_name);
        item.os_version = Some(info.os_version);
        item.os_arch = Some(info.os_arch);
        item.cpu_name = Some(info.cpu_name);
        item.cpu_vendor = Some(info.cpu_vendor);
        item.cpu_frequency = Some(info.cpu_frequency as i64);
        item.cpu_nums = Some(info.cpu_nums as i32);
        item.gmt_modified = Some(now);
        let _r = TbAigw::update_by_id(rb, &item, item.id.unwrap()).await;
    }
    // insert new item
    else {
        let item = TbAigw {
            id: None,
            cluster_name: Some(info.cluster),
            ip: Some(info.ip),
            version: Some(info.version),
            os_name: Some(info.os_name),
            os_version: Some(info.os_version),
            os_arch: Some(info.os_arch),
            cpu_name: Some(info.cpu_name),
            cpu_vendor: Some(info.cpu_vendor),
            cpu_frequency: Some(info.cpu_frequency as i64),
            cpu_nums: Some(info.cpu_nums as i32),
            gmt_create: Some(now),
            gmt_modified: Some(now),
        };
        let _ = TbAigw::insert(rb, &item).await?;
    }

    Ok(())
}

pub async fn find_aigw_by_page(
    rb: &sqlx::MySqlPool,
    page_request: &PageRequest,
    cluster_name: &str,
) -> anyhow::Result<Page<Server>> {
    let r = TbAigw::select_by_page(rb, page_request, cluster_name).await?;
    let mut page = Page::new(r.page_no, r.page_size, r.total, vec![]);
    for tb_server in r.records {
        let server = convert_tb_aigw(&tb_server);
        page.items.push(server);
    }
    Ok(page)
}

fn convert_tb_aigw(server: &TbAigw) -> Server {
    let gmt_create = server
        .gmt_create
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_HH_MM_SS_FORMAT));

    let gmt_modified = server
        .gmt_modified
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_HH_MM_SS_FORMAT));
    Server {
        id: server.id.map(|id| id as u64),
        cluster_name: server
            .cluster_name
            .clone()
            .unwrap_or("".to_string()),
        ip: server.ip.clone().unwrap_or("".to_string()),
        version: server.version.clone().unwrap_or("".to_string()),
        os_name: server.os_name.clone().unwrap_or("".to_string()),
        os_version: server.os_version.clone().unwrap_or("".to_string()),
        os_arch: server.os_arch.clone().unwrap_or("".to_string()),
        cpu_name: server
            .cpu_name
            .clone()
            .unwrap_or("".to_string()),
        cpu_vendor: server
            .cpu_vendor
            .clone()
            .unwrap_or("".to_string()),
        cpu_frequency: server.cpu_frequency.map_or(1, |i| i as u64),
        cpu_nums: server.cpu_nums.map_or(1, |i| i as u32),
        gmt_create,
        gmt_modified,
    }
}

#[derive(Serialize, Deserialize)]
pub struct Server {
    pub id: Option<u64>,
    pub cluster_name: String,
    pub ip: String,
    pub version: String,
    pub os_name: String,
    pub os_version: String,
    pub os_arch: String,
    pub cpu_name: String,
    pub cpu_vendor: String,
    pub cpu_frequency: u64,
    pub cpu_nums: u32,
    pub gmt_create: Option<String>,
    pub gmt_modified: Option<String>,
}
