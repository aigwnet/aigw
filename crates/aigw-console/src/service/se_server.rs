use aigw_core::{HandshakeInfo, date_format_local};
use rbatis::{IPageRequest, RBatis, rbdc::DateTime};
use serde::{Deserialize, Serialize};

use crate::{
    service::{Page, YYYY_MM_DD_HH_MM_SS_FORMAT},
    storage::tb_server::TbServer,
};

pub async fn update_or_insert_server(
    rb: &rbatis::RBatis,
    info: HandshakeInfo,
) -> anyhow::Result<()> {
    let now = DateTime::utc();
    let item = TbServer::select_by_cluster_name_and_ip(rb, &info.cluster, &info.ip).await?;
    // update last_active_time
    if let Some(mut item) = item {
        item.version = Some(info.version);
        item.os_name = Some(info.os_name);
        item.os_version = Some(info.os_version);
        item.os_arch = Some(info.os_arch);
        item.cpu_name = Some(info.cpu_name);
        item.cpu_vendor = Some(info.cpu_vendor);
        item.cpu_frequency = Some(info.cpu_frequency);
        item.cpu_nums = Some(info.cpu_nums);
        item.gmt_modified = Some(now);
        let _r = TbServer::update_by_id(rb, &item, item.id.unwrap()).await;
    }
    // insert new item
    else {
        let item = TbServer {
            id: None,
            cluster_name: Some(info.cluster),
            ip: Some(info.ip),
            version: Some(info.version),
            os_name: Some(info.os_name),
            os_version: Some(info.os_version),
            os_arch: Some(info.os_arch),
            cpu_name: Some(info.cpu_name),
            cpu_vendor: Some(info.cpu_vendor),
            cpu_frequency: Some(info.cpu_frequency),
            cpu_nums: Some(info.cpu_nums),
            gmt_create: Some(now.clone()),
            gmt_modified: Some(now),
        };
        let _ = TbServer::insert(rb, &item).await?;
    }

    Ok(())
}

pub async fn find_server_by_page(
    rb: &RBatis,
    page_request: &dyn IPageRequest,
    cluster_name: &str,
) -> anyhow::Result<Page<Server>> {
    let r: rbatis::Page<TbServer> =
        TbServer::select_by_page(rb, page_request, cluster_name).await?;
    let mut page = Page::new(r.page_no, r.page_size, r.total, vec![]);
    for tb_server in r.records {
        let server = convert_tb_server(&tb_server);
        page.items.push(server);
    }
    Ok(page)
}

fn convert_tb_server(server: &TbServer) -> Server {
    let gmt_create = server
        .gmt_create
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_HH_MM_SS_FORMAT));

    let gmt_modified = server
        .gmt_modified
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_HH_MM_SS_FORMAT));
    Server {
        id: server.id,
        cluster_name: server
            .cluster_name
            .clone()
            .map_or("".to_string(), |name| name),
        ip: server.ip.clone().map_or("".to_string(), |s| s),
        version: server.version.clone().map_or("".to_string(), |s| s),
        os_name: server.os_name.clone().map_or("".to_string(), |s| s),
        os_version: server.os_version.clone().map_or("".to_string(), |s| s),
        os_arch: server.os_arch.clone().map_or("".to_string(), |s| s),
        cpu_name: server
            .cpu_name
            .clone()
            .map_or("".to_string(), |s: String| s),
        cpu_vendor: server
            .cpu_vendor
            .clone()
            .map_or("".to_string(), |s: String| s),
        cpu_frequency: server.cpu_frequency.map_or(1, |i| i),
        cpu_nums: server.cpu_nums.map_or(1, |i| i),
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
