use crate::{
    service::{Page, YYYY_MM_DD_HH_MM_SS_FORMAT, do_build_change_log},
    storage::{PageRequest, tb_cluster::TbCluster},
};
use aigw_core::{ChangeLog, Cluster, LogAction, LogType, date_format_local};
use time::OffsetDateTime;

pub async fn add_cluster(rb: &sqlx::MySqlPool, cluster: &Cluster) -> anyhow::Result<ChangeLog> {
    let now = OffsetDateTime::now_utc();
    TbCluster::insert(
        rb,
        &TbCluster {
            id: None,
            name: Some(cluster.name.clone()),
            security_key: Some(cluster.security_key.clone()),
            enable: cluster.enable,
            enable_default_site: cluster.enable_default_site,
            enable_white_list: cluster.enable_white_list,
            enable_block_list: cluster.enable_block_list,
            description: cluster.description.clone(),
            gmt_create: Some(now),
            gmt_modified: Some(now),
        },
    )
    .await?;

    let c = find_cluster_by_name(rb, &cluster.name).await?;
    let s = serde_json::to_string_pretty(&c)?;
    let mut conn = rb.acquire().await?;
    let change_log = do_build_change_log(
        &mut conn,
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

pub async fn modify_cluster(
    rb: &sqlx::MySqlPool,
    cluster: &Cluster,
    name: &str,
) -> anyhow::Result<ChangeLog> {
    let now = OffsetDateTime::now_utc();
    let table = &TbCluster {
        id: None,
        name: Some(cluster.name.clone()),
        security_key: Some(cluster.security_key.clone()),
        enable: cluster.enable,
        enable_default_site: cluster.enable_default_site,
        enable_white_list: cluster.enable_white_list,
        enable_block_list: cluster.enable_block_list,
        description: cluster.description.clone(),
        gmt_create: None,
        gmt_modified: Some(now),
    };
    TbCluster::update_by_name(rb, table, name).await?;
    let c = find_cluster_by_name(rb, &cluster.name).await?;
    let s = serde_json::to_string_pretty(&c)?;
    let mut conn = rb.acquire().await?;
    let change_log = do_build_change_log(
        &mut conn,
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

pub async fn find_cluster(rb: &sqlx::MySqlPool, name: &str) -> anyhow::Result<Cluster> {
    let cluster = TbCluster::select_by_name(rb, name)
        .await?
        .ok_or(anyhow::anyhow!("Cluster not found."))?;
    Ok(convert_tb_cluster(&cluster))
}

pub async fn find_cluster_by_name(
    rb: &sqlx::MySqlPool,
    name: &str,
) -> anyhow::Result<Cluster> {
    let cluster = TbCluster::select_by_name(rb, name)
        .await?
        .ok_or(anyhow::anyhow!("Cluster not found."))?;
    Ok(convert_tb_cluster(&cluster))
}

pub async fn find_all(rb: &sqlx::MySqlPool) -> anyhow::Result<Vec<Cluster>> {
    let clusters = TbCluster::select_all(rb)
        .await?
        .iter()
        .map(convert_tb_cluster)
        .collect();

    Ok(clusters)
}

pub async fn delete_cluster(rb: &sqlx::MySqlPool, name: &str) -> anyhow::Result<()> {
    let _ = TbCluster::delete_by_name(rb, name).await?;
    Ok(())
}

pub async fn find_cluster_by_page(
    rb: &sqlx::MySqlPool,
    page_request: &PageRequest,
) -> anyhow::Result<Page<Cluster>> {
    let r = TbCluster::select_page(rb, page_request).await?;
    let mut page = Page::new(r.page_no, r.page_size, r.total, vec![]);
    for cluster in r.records {
        let cluster = convert_tb_cluster(&cluster);
        page.items.push(cluster);
    }
    Ok(page)
}

fn convert_tb_cluster(cluster: &TbCluster) -> Cluster {
    let gmt_modified = cluster
        .gmt_modified
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_HH_MM_SS_FORMAT));
    Cluster {
        id: cluster.id.map(|id| id as u64),
        name: cluster.name.clone().unwrap_or("".to_string()),
        security_key: cluster
            .security_key
            .clone()
            .unwrap_or("".to_string()),
        enable: cluster.enable,
        enable_default_site: cluster.enable_default_site,
        enable_white_list: cluster.enable_white_list,
        enable_block_list: cluster.enable_block_list,
        real_ip_from: vec![],
        description: cluster.description.clone(),
        gmt_modified,
    }
}
