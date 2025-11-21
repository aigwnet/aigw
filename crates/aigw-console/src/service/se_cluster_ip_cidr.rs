use aigw_core::{ChangeLog, IpUpdate, IpUpdateList, LogAction, LogType, date_format_local};
use rbatis::{IPageRequest, RBatis, rbdc::DateTime};
use serde::{Deserialize, Serialize};

use crate::{
    service::{Page, YYYY_MM_DD_HH_MM_SS_FORMAT, do_build_change_log},
    storage::tb_cluster_ip_cidr::TbClusterIpCidr,
};

#[derive(Serialize, Deserialize)]
pub struct ClusterIpCidr {
    pub id: Option<u64>,
    pub cluster_name: String,
    pub ip: String,
    pub prefix_len: u32,
    pub r#type: u8,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub gmt_modified: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct IpCidr {
    pub ip: String,
    pub prefix_len: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ClusterIpCidrList {
    pub id: Option<u64>,
    pub cluster_name: String,
    pub list: Vec<IpCidr>,
    pub r#type: u8,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub gmt_modified: Option<String>,
}

pub async fn add_new_cluster_ip(
    rb: &RBatis,
    list: &ClusterIpCidrList,
) -> anyhow::Result<ChangeLog> {
    let now = DateTime::utc();

    let mut update_list = vec![];
    for ip_cidr in &list.list {
        TbClusterIpCidr::insert(
            rb,
            &TbClusterIpCidr {
                id: None,
                cluster_name: Some(list.cluster_name.clone()),
                ip: Some(ip_cidr.ip.clone()),
                prefix_len: Some(ip_cidr.prefix_len),
                r#type: Some(list.r#type),
                start_time: None,
                end_time: None,
                gmt_create: Some(now.clone()),
                gmt_modified: Some(now.clone()),
            },
        )
        .await?;

        update_list.push(IpUpdate {
            start_time: 0,
            end_time: 0,
            prefix_len: ip_cidr.prefix_len,
            data: ip_cidr.ip.clone(),
        });
    }

    let data = IpUpdateList {
        item_type: list.r#type.into(),
        data: update_list,
    };

    let s = serde_json::to_string_pretty(&data)?;
    let change_log = do_build_change_log(
        rb,
        list.cluster_name.clone(),
        LogType::IpLayer4,
        LogAction::Add,
        0,
        0,
        Some(s),
    )
    .await?;
    Ok(change_log)
}

pub async fn find_ip_cidr_by_page(
    rb: &RBatis,
    page_request: &dyn IPageRequest,
    cluster_name: &str,
    r#type: u8,
) -> anyhow::Result<Page<ClusterIpCidr>> {
    let r = TbClusterIpCidr::select_page(rb, page_request, cluster_name, r#type).await?;
    let mut page = Page::new(r.page_no, r.page_size, r.total, vec![]);
    for tb_cluster_ip_cidr in r.records {
        let r = convert_tb_cluster_ip_cidr(tb_cluster_ip_cidr);
        page.items.push(r);
    }
    Ok(page)
}

pub async fn delete_cluster_ip(rb: &RBatis, id: u64) -> anyhow::Result<()> {
    let _ = TbClusterIpCidr::delete_by_id(rb, id).await?;
    Ok(())
}

fn convert_tb_cluster_ip_cidr(tb_cluster_ip_cidr: TbClusterIpCidr) -> ClusterIpCidr {
    let start_time = tb_cluster_ip_cidr
        .start_time
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_HH_MM_SS_FORMAT));

    let end_time = tb_cluster_ip_cidr
        .end_time
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_HH_MM_SS_FORMAT));

    let gmt_modified = tb_cluster_ip_cidr
        .gmt_modified
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_HH_MM_SS_FORMAT));

    ClusterIpCidr {
        id: tb_cluster_ip_cidr.id,
        cluster_name: tb_cluster_ip_cidr.cluster_name.map_or("".to_owned(), |s| s),
        ip: tb_cluster_ip_cidr.ip.map_or("".to_owned(), |s| s),
        prefix_len: tb_cluster_ip_cidr.prefix_len.map_or(0, |i| i),
        r#type: tb_cluster_ip_cidr.r#type.map_or(0, |i| i),
        start_time,
        end_time,
        gmt_modified,
    }
}
