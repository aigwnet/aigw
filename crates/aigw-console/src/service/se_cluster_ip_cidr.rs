use rbatis::{IPageRequest, RBatis};
use serde::{Deserialize, Serialize};

use crate::{service::Page, storage::tb_cluster_ip_cidr::TbClusterIpCidr};

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

fn convert_tb_cluster_ip_cidr(tb_cluster_ip_cidr: TbClusterIpCidr) -> ClusterIpCidr {
    let start_time = tb_cluster_ip_cidr.start_time.as_ref().map_or(None, |s| {
        chrono::DateTime::from_timestamp(s.unix_timestamp(), 0).map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
    });
    let end_time = tb_cluster_ip_cidr.end_time.as_ref().map_or(None, |s| {
        chrono::DateTime::from_timestamp(s.unix_timestamp(), 0).map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
    });
    let gmt_modified = tb_cluster_ip_cidr.gmt_modified.as_ref().map_or(None, |s| {
        chrono::DateTime::from_timestamp(s.unix_timestamp(), 0).map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
    });

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
