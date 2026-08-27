use aigw_core::{ChangeLog, IpItem, IpList, LogAction, LogType, date_format_local};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    service::{Page, YYYY_MM_DD_HH_MM_SS_FORMAT, do_build_change_log},
    storage::{PageRequest, tb_cluster_ip_cidr::TbClusterIpCidr},
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
    rb: &sqlx::MySqlPool,
    list: &ClusterIpCidrList,
) -> anyhow::Result<ChangeLog> {
    let now = OffsetDateTime::now_utc();

    let mut ip_list = vec![];
    for ip_cidr in &list.list {
        TbClusterIpCidr::insert(
            rb,
            &TbClusterIpCidr {
                id: None,
                cluster_name: Some(list.cluster_name.clone()),
                ip: Some(ip_cidr.ip.clone()),
                prefix_len: Some(ip_cidr.prefix_len as i32),
                r#type: Some(list.r#type as i8),
                start_time: None,
                end_time: None,
                gmt_create: Some(now),
                gmt_modified: Some(now),
            },
        )
        .await?;

        ip_list.push(IpItem {
            prefix_len: ip_cidr.prefix_len,
            data: ip_cidr.ip.clone(),
        });
    }

    let data = IpList {
        item_type: list.r#type.into(),
        data: ip_list,
    };

    let s = serde_json::to_string_pretty(&data)?;
    let mut conn = rb.acquire().await.map_err(|e| anyhow::anyhow!(e))?;
    let change_log = do_build_change_log(
        &mut conn,
        list.cluster_name.clone(),
        LogType::IpLayer4,
        LogAction::Create,
        0,
        0,
        Some(s),
    )
    .await?;
    Ok(change_log)
}

pub async fn find_ip_cidr_by_page(
    rb: &sqlx::MySqlPool,
    page_request: &PageRequest,
    cluster_name: &str,
    r#type: u8,
) -> anyhow::Result<Page<ClusterIpCidr>> {
    let r = TbClusterIpCidr::select_page(rb, page_request, cluster_name, r#type as i8).await?;
    let mut page = Page::new(r.page_no, r.page_size, r.total, vec![]);
    for tb_cluster_ip_cidr in r.records {
        let r = convert_tb_cluster_ip_cidr(tb_cluster_ip_cidr);
        page.items.push(r);
    }
    Ok(page)
}

pub async fn delete_cluster_ip(rb: &sqlx::MySqlPool, id: u64) -> anyhow::Result<ChangeLog> {
    let ip = TbClusterIpCidr::select_by_id(rb, id as i64)
        .await?
        .ok_or(anyhow::anyhow!("ClusterIpCidr not found."))?;
    let _ = TbClusterIpCidr::delete_by_id(rb, id as i64).await?;

    let data = IpList {
        item_type: ip.r#type.unwrap_or_default() as u32,
        data: vec![IpItem {
            prefix_len: ip.prefix_len.unwrap_or_default() as u32,
            data: ip.ip.unwrap_or_default(),
        }],
    };

    let s = serde_json::to_string_pretty(&data)?;
    let mut conn = rb.acquire().await.map_err(|e| anyhow::anyhow!(e))?;
    let change_log = do_build_change_log(
        &mut conn,
        ip.cluster_name.unwrap_or_default(),
        LogType::IpLayer4,
        LogAction::Delete,
        0,
        0,
        Some(s),
    )
    .await?;
    Ok(change_log)
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
        id: tb_cluster_ip_cidr.id.map(|id| id as u64),
        cluster_name: tb_cluster_ip_cidr.cluster_name.unwrap_or("".to_owned()),
        ip: tb_cluster_ip_cidr.ip.unwrap_or("".to_owned()),
        prefix_len: tb_cluster_ip_cidr.prefix_len.map_or(0, |i| i as u32),
        r#type: tb_cluster_ip_cidr.r#type.map_or(0, |i| i as u8),
        start_time,
        end_time,
        gmt_modified,
    }
}
