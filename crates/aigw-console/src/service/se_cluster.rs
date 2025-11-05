use aigw_core::{ChangeLog, Cluster, LogAction, LogType};
use rbatis::{IPageRequest, RBatis, rbdc::DateTime};

use crate::{
    service::{Page, do_build_change_log},
    storage::tb_cluster::TbCluster,
};

pub async fn add_new_cluster(rb: &RBatis, cluster: &Cluster) -> anyhow::Result<ChangeLog> {
    let now = DateTime::utc();
    TbCluster::insert(
        rb,
        &TbCluster {
            id: None,
            name: Some(cluster.name.clone()),
            security_key: Some(cluster.security_key.clone()),
            enable: cluster.enable,
            default_site_enable: cluster.default_site_enable,
            description: cluster.description.clone(),
            gmt_create: Some(now.clone()),
            gmt_modified: Some(now),
        },
    )
    .await?;

    let c = find_cluster_by_name(rb, &cluster.name).await?;
    let s = serde_json::to_string_pretty(&c)?;
    let change_log = do_build_change_log(
        rb,
        cluster.name.clone(),
        LogType::Cluster,
        LogAction::Update,
        c.id.unwrap_or_default(),
        0,
        Some(s),
    )
    .await?;
    Ok(change_log)
}

pub async fn modify_cluster(rb: &RBatis, cluster: &Cluster, id: u64) -> anyhow::Result<ChangeLog> {
    let now = DateTime::utc();
    let table = &TbCluster {
        id: None,
        name: Some(cluster.name.clone()),
        security_key: Some(cluster.security_key.clone()),
        enable: cluster.enable,
        default_site_enable: cluster.default_site_enable,
        description: cluster.description.clone(),
        gmt_create: None,
        gmt_modified: Some(now),
    };
    TbCluster::update_by_id(rb, table, id).await?;
    let c = find_cluster_by_name(rb, &cluster.name).await?;
    let s = serde_json::to_string_pretty(&c)?;
    let change_log = do_build_change_log(
        rb,
        cluster.name.clone(),
        LogType::Cluster,
        LogAction::Update,
        id,
        0,
        Some(s),
    )
    .await?;
    Ok(change_log)
}

pub async fn find_cluster(rb: &RBatis, id: u64) -> anyhow::Result<Cluster> {
    let cluster = TbCluster::select_by_id(rb, id)
        .await?
        .ok_or(anyhow::anyhow!("Cluster not found."))?;
    Ok(convert_tb_cluster(&cluster))
}

pub async fn find_cluster_by_name(rb: &RBatis, name: &str) -> anyhow::Result<Cluster> {
    let cluster = TbCluster::select_by_name(rb, name)
        .await?
        .ok_or(anyhow::anyhow!("Cluster not found."))?;
    Ok(convert_tb_cluster(&cluster))
}

pub async fn find_all(rb: &RBatis) -> anyhow::Result<Vec<Cluster>> {
    let clusters = TbCluster::select_all(rb)
        .await?
        .iter()
        .map(|cluster| convert_tb_cluster(cluster))
        .collect();

    Ok(clusters)
}

pub async fn delete_cluster(rb: &RBatis, name: &str) -> anyhow::Result<()> {
    let _ = TbCluster::delete_by_name(rb, name).await?;
    Ok(())
}

pub async fn find_cluster_by_page(
    rb: &RBatis,
    page_request: &dyn IPageRequest,
) -> anyhow::Result<Page<Cluster>> {
    let r: rbatis::Page<TbCluster> = TbCluster::select_page(rb, page_request).await?;
    let mut page = Page::new(r.page_no, r.page_size, r.total, vec![]);
    for cluster in r.records {
        let cluster = convert_tb_cluster(&cluster);
        page.items.push(cluster);
    }
    Ok(page)
}

fn convert_tb_cluster(cluster: &TbCluster) -> Cluster {
    let gmt_modified = cluster.gmt_modified.as_ref().map_or(None, |s| {
        chrono::DateTime::from_timestamp(s.unix_timestamp(), 0).map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
    });
    Cluster {
        id: cluster.id,
        name: cluster.name.clone().map_or("".to_string(), |name| name),
        security_key: cluster
            .security_key
            .clone()
            .map_or("".to_string(), |key| key),
        enable: cluster.enable,
        default_site_enable: cluster.default_site_enable,
        description: cluster.description.clone(),
        gmt_modified,
    }
}
