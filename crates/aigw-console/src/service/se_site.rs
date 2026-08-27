use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, OnceLock},
};

use aigw_core::{
    ChangeLog, DynamicCert, HttpVersion, LogAction, LogType, ProxyLocation, Site, TlsPrivateKey,
    convert_headers, convert_headers_to_string, date_format_local, new_path_selector, new_rewrite,
};
use http::HeaderName;
use time::OffsetDateTime;

use crate::{
    service::{Page, YYYY_MM_DD_FORMAT, do_build_change_log},
    storage::{
        PageRequest, tb_backend::TbBackend, tb_change_log::TbChangeLog, tb_location::TbLocation,
        tb_site::TbSite,
    },
};

pub async fn add_site(rb: &sqlx::MySqlPool, site: &Site) -> anyhow::Result<(Site, ChangeLog)> {
    if site.locations.is_empty() {
        return Err(anyhow::anyhow!("Location is empty"));
    }

    for location in &site.locations {
        if location.proxy && location.upstream.is_empty() {
            let err = "Location '".to_string() + location.path.as_str() + "' backends is empty.";
            return Err(anyhow::anyhow!(err));
        }
    }
    let mut tx = rb.begin().await?;
    match do_add_new_site(&mut tx, site).await {
        Ok(c) => {
            tx.commit().await?;
            Ok(c)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

pub async fn modify_site(rb: &sqlx::MySqlPool, site: &Site) -> anyhow::Result<(Site, ChangeLog)> {
    if site.locations.is_empty() {
        return Err(anyhow::anyhow!("Location is empty"));
    }

    for location in &site.locations {
        if location.proxy && location.upstream.is_empty() {
            let err = "Location '".to_string() + location.path.as_str() + "' backends is empty.";
            return Err(anyhow::anyhow!(err));
        }
    }
    let mut tx = rb.begin().await?;
    match do_modify_site(&mut tx, site).await {
        Ok(c) => {
            tx.commit().await?;
            Ok(c)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

pub async fn update_cert(
    conn: &mut sqlx::MySqlConnection,
    name: &str,
    tls_cert: String,
    tls_private_key: String,
) -> anyhow::Result<(Site, ChangeLog)> {
    let r = TbSite::select_by_name(&mut *conn, name).await?;
    let mut tb_site = r.ok_or(anyhow::anyhow!("Site not found."))?;

    let now = OffsetDateTime::now_utc();

    let cert = DynamicCert::try_from(tls_cert.as_bytes())?;

    tb_site.tls_cert = Some(tls_cert);
    tb_site.tls_cert_start_date = Some(
        OffsetDateTime::from_unix_timestamp(cert.cert.not_before().unix_timestamp())
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
    );
    tb_site.tls_cert_end_date = Some(
        OffsetDateTime::from_unix_timestamp(cert.cert.not_after().unix_timestamp())
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
    );
    tb_site.tls_private_key = Some(tls_private_key);
    tb_site.gmt_modified = Some(now);

    // update site
    TbSite::update_by_name(&mut *conn, &tb_site, name).await?;

    let (id, site) = find_site(&mut *conn, name).await?;
    let s = serde_json::to_string_pretty(&site)?;
    let change_log = do_build_change_log(
        &mut *conn,
        site.cluster.clone(),
        LogType::Site,
        LogAction::Update,
        id,
        0,
        Some(s),
    )
    .await?;

    Ok((site, change_log))
}

pub async fn find_site(
    conn: &mut sqlx::MySqlConnection,
    name: &str,
) -> anyhow::Result<(u64, Site)> {
    let tb_site = TbSite::select_by_name(&mut *conn, name).await?;
    if let Some(server) = tb_site {
        let id: u64 = server.id.unwrap_or_default() as u64;
        let site = convert_tb_site(&mut *conn, server).await?;
        return Ok((id, site));
    }
    Err(anyhow::anyhow!("Resource not found"))
}

pub async fn build_change_log_delete_site(
    rb: &sqlx::MySqlPool,
    name: &str,
) -> anyhow::Result<ChangeLog> {
    let mut conn = rb.acquire().await?;
    let (id, site) = find_site(&mut conn, name).await?;
    drop(conn);
    let s = serde_json::to_string_pretty(&site)?;
    let mut tx = rb.begin().await?;

    TbChangeLog::delete_by_data_id(&mut *tx, id as i64).await?;
    // 1. delete site
    match TbSite::delete_by_name(&mut *tx, name).await {
        Ok(r) => {
            if r.rows_affected() < 1 {
                tx.commit().await?;
                return Err(anyhow::anyhow!("Resource not found"));
            }
        }
        Err(e) => {
            tx.rollback().await?;
            return Err(anyhow::anyhow!(e));
        }
    }
    // 2. build change log
    match do_build_change_log(
        &mut tx,
        site.cluster.clone(),
        LogType::Site,
        LogAction::Delete,
        id,
        0,
        Some(s),
    )
    .await
    {
        Ok(item) => {
            tx.commit().await?;
            Ok(item)
        }
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

async fn convert_tb_site(
    conn: &mut sqlx::MySqlConnection,
    tb_site: TbSite,
) -> anyhow::Result<Site> {
    //
    let tls_private_key = tb_site
        .tls_private_key
        .and_then(|item| TlsPrivateKey::try_from(item.as_bytes()).ok());
    let tls_cert = tb_site
        .tls_cert
        .and_then(|item| DynamicCert::try_from(item.as_bytes()).ok());
    let tls_cert_start_date = tb_site
        .tls_cert_start_date
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_FORMAT));
    let tls_cert_end_date = tb_site
        .tls_cert_end_date
        .as_ref()
        .and_then(|d| date_format_local(d.unix_timestamp(), YYYY_MM_DD_FORMAT));
    let cluster = tb_site
        .cluster_name
        .clone()
        .ok_or(anyhow::anyhow!("Cluster is null"))?;

    let name = tb_site.name.ok_or(anyhow::anyhow!("Name is null"))?;

    let mut site = Site {
        id: tb_site.id.map(|id| id as u64),
        cluster,
        name: name.clone(),
        alt_names: vec![],
        auto_index: tb_site.auto_index,
        root_dir: tb_site.root_dir.map(|item| item.into()),
        tls_on: tb_site.tls_on,
        tls_enforce: tb_site.tls_enforce,
        acme_on: tb_site.acme_on,
        tls_cert,
        tls_cert_start_date,
        tls_cert_end_date,
        tls_private_key,
        rate_limit: tb_site.rate_limit.map_or(0, |i| i as isize),
        rate_limit_unit: tb_site
            .rate_limit_unit
            .map_or(1000, |i| if i == 0 { 1000 } else { i as u64 }),
        locations: vec![],
        certified_key: None,
    };

    if let Some(names) = tb_site.alt_names
        && !names.is_empty()
    {
        let names = names.split(",").collect::<Vec<&str>>();
        for name in names {
            site.alt_names.push(name.to_owned());
        }
    }
    let site_id = tb_site.id.ok_or(anyhow::anyhow!("Id is null"))?;

    let locations = TbLocation::select_by_site_id(&mut *conn, site_id).await?;
    for location in locations {
        let location_id = location.id.ok_or(anyhow::anyhow!("Location Id is null"))?;
        let mut backends_array = vec![];
        let backends = TbBackend::select_by_location_id(&mut *conn, location_id).await?;
        for backend in backends {
            let b = backend.host.unwrap() + ":" + backend.port.unwrap().to_string().as_str();
            backends_array.push(b);
        }

        let p_location = ProxyLocation {
            id: Some(location_id as u64),
            path: Arc::new(new_path_selector(&location.location.unwrap())?),
            proxy: location.proxy == Some(1),
            protocol: location.protocol.unwrap().as_str().try_into()?,
            lb: OnceLock::new(),
            upstream: backends_array,
            connection_timeout: location.connection_timeout.map_or(5, |t| t as u32),
            read_timeout: location.read_timeout.map_or(5, |t| t as u32),
            write_timeout: location.write_timeout.map_or(5, |t| t as u32),
            idle_timeout: location.idle_timeout.map_or(60, |t| t as u32),
            sni: location.sni.unwrap_or("$host".to_string()),
            client_max_body_size: location.client_max_body_size.map_or(0, |t| t as usize),
            rewrite: new_rewrite(location.rewrite.as_deref())?,
            http_version: location
                .http_version
                .as_ref()
                .and_then(|s| HttpVersion::try_from(s.as_str()).ok()),
            proxy_add_headers: location.proxy_add_headers.and_then(|s| {
                if let Ok(headers) = serde_json::from_str::<Vec<HashMap<String, String>>>(&s) {
                    convert_headers(&headers).ok()
                } else {
                    Some(vec![])
                }
            }),
            proxy_set_headers: location.proxy_set_headers.and_then(|s| {
                if let Ok(headers) = serde_json::from_str::<Vec<HashMap<String, String>>>(&s) {
                    convert_headers(&headers).ok()
                } else {
                    Some(vec![])
                }
            }),
            proxy_remove_headers: location.proxy_remove_headers.map(|s| {
                if let Ok(headers) = serde_json::from_str::<Vec<String>>(&s) {
                    let mut r = vec![];
                    for h in &headers {
                        if let Ok(h) = HeaderName::from_lowercase(h.to_lowercase().as_bytes()) {
                            r.push(h);
                        }
                    }
                    r
                } else {
                    vec![]
                }
            }),
            response_add_headers: location.response_add_headers.and_then(|s| {
                if let Ok(headers) = serde_json::from_str::<Vec<HashMap<String, String>>>(&s) {
                    convert_headers(&headers).ok()
                } else {
                    Some(vec![])
                }
            }),
            response_set_headers: location.response_set_headers.and_then(|s| {
                if let Ok(headers) = serde_json::from_str::<Vec<HashMap<String, String>>>(&s) {
                    convert_headers(&headers).ok()
                } else {
                    Some(vec![])
                }
            }),
            response_remove_headers: location.response_remove_headers.map(|s| {
                if let Ok(headers) = serde_json::from_str::<Vec<String>>(&s) {
                    let mut r = vec![];
                    for h in &headers {
                        if let Ok(h) = HeaderName::from_lowercase(h.to_lowercase().as_bytes()) {
                            r.push(h);
                        }
                    }
                    r
                } else {
                    vec![]
                }
            }),
            root_dir: location.root_dir.map(|item| item.into()),
            auto_index: location.auto_index == Some(1),
        };

        site.locations.push(Arc::new(p_location));
    }
    Ok(site)
}

pub async fn find_site_by_page(
    rb: &sqlx::MySqlPool,
    page_request: &PageRequest,
    cluster_name: &str,
) -> anyhow::Result<Page<Site>> {
    let r = TbSite::select_page(rb, page_request, cluster_name).await?;
    let mut page = Page::new(r.page_no, r.page_size, r.total, vec![]);
    let mut conn = rb.acquire().await?;
    for tb_site in r.records {
        let site = convert_tb_site(&mut conn, tb_site).await?;
        page.items.push(site);
    }
    Ok(page)
}

async fn do_add_new_site(
    conn: &mut sqlx::MySqlConnection,
    site: &Site,
) -> anyhow::Result<(Site, ChangeLog)> {
    let now = OffsetDateTime::now_utc();

    let tls_private_key = if site.acme_on {
        None
    } else {
        site.tls_private_key
            .as_ref()
            .and_then(|item| item.try_to_string().ok())
    };
    let tls_cert = if site.acme_on {
        None
    } else {
        site.tls_cert
            .as_ref()
            .and_then(|item| item.try_to_string().ok())
    };
    let (tls_cert_start_date, tls_cert_end_date) = if site.acme_on {
        (None, None)
    } else {
        site.tls_cert.as_ref().map_or((None, None), |cert| {
            (
                Some(
                    OffsetDateTime::from_unix_timestamp(cert.cert.not_before().unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
                Some(
                    OffsetDateTime::from_unix_timestamp(cert.cert.not_after().unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
            )
        })
    };

    let tb_site = TbSite {
        id: None,
        cluster_name: Some(site.cluster.clone()),
        name: Some(site.name.clone()),
        alt_names: Some(site.alt_names.join(",")),
        root_dir: site.root_dir.as_ref().map(|item| unsafe {
            String::from_utf8_unchecked(item.as_os_str().as_encoded_bytes().to_vec())
        }),
        auto_index: site.auto_index,
        tls_on: site.tls_on,
        tls_enforce: site.tls_enforce,
        acme_on: site.acme_on,
        tls_cert,
        tls_cert_start_date,
        tls_cert_end_date,
        tls_private_key,
        rate_limit: Some(site.rate_limit as i64),
        rate_limit_unit: Some(site.rate_limit_unit as i64),
        gmt_create: Some(now),
        gmt_modified: Some(now),
    };
    let r = TbSite::insert(&mut *conn, &tb_site).await?;
    let site_id = r.last_insert_id() as i64;
    for location in &site.locations {
        let tb_location = convert_location(site_id, location, &now);
        let r = TbLocation::insert(&mut *conn, &tb_location).await?;
        let location_id = r.last_insert_id() as i64;
        let to_be_add: HashSet<(String, u16)> = location
            .upstream
            .iter()
            .filter(|item| item.contains(":"))
            .filter_map(|s| {
                let mut parts = s.split(':');
                let ip = parts.next()?.to_string();
                let port = parts.next()?.parse::<u16>().ok()?;
                Some((ip, port))
            })
            .collect();
        for (host, port) in to_be_add.iter() {
            add_backend(&mut *conn, location_id, host, *port, &now).await?;
        }
    }

    let (id, site) = find_site(&mut *conn, &site.name).await?;
    let s = serde_json::to_string_pretty(&site)?;
    let change_log = do_build_change_log(
        &mut *conn,
        site.cluster.clone(),
        LogType::Site,
        LogAction::Create,
        id,
        0,
        Some(s),
    )
    .await?;

    Ok((site, change_log))
}

async fn do_modify_site(
    conn: &mut sqlx::MySqlConnection,
    site: &Site,
) -> anyhow::Result<(Site, ChangeLog)> {
    let r = TbSite::select_by_name(&mut *conn, &site.name).await?;
    let mut tb_site = r.ok_or(anyhow::anyhow!("Server not found."))?;
    let site_id = tb_site.id.ok_or(anyhow::anyhow!("Server id not found."))?;
    let now = OffsetDateTime::now_utc();

    let tls_private_key = if site.acme_on {
        None
    } else {
        site.tls_private_key
            .as_ref()
            .and_then(|item| item.try_to_string().ok())
    };
    let tls_cert = if site.acme_on {
        None
    } else {
        site.tls_cert
            .as_ref()
            .and_then(|item| item.try_to_string().ok())
    };

    let (tls_cert_start_date, tls_cert_end_date) = if site.acme_on {
        (None, None)
    } else {
        site.tls_cert.as_ref().map_or((None, None), |cert| {
            (
                Some(
                    OffsetDateTime::from_unix_timestamp(cert.cert.not_before().unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
                Some(
                    OffsetDateTime::from_unix_timestamp(cert.cert.not_after().unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
            )
        })
    };

    tb_site.cluster_name = Some(site.cluster.clone());
    tb_site.alt_names = Some(site.alt_names.join(","));
    tb_site.auto_index = site.auto_index;
    tb_site.root_dir = site.root_dir.as_ref().map(|item| unsafe {
        String::from_utf8_unchecked(item.as_os_str().as_encoded_bytes().to_vec())
    });
    tb_site.tls_cert = tls_cert;
    tb_site.tls_cert_start_date = tls_cert_start_date;
    tb_site.tls_cert_end_date = tls_cert_end_date;
    tb_site.tls_private_key = tls_private_key;
    tb_site.rate_limit = Some(site.rate_limit as i64);
    tb_site.rate_limit_unit = Some(site.rate_limit_unit as i64);
    tb_site.tls_on = site.tls_on;
    tb_site.tls_enforce = site.tls_enforce;
    tb_site.acme_on = site.acme_on;
    tb_site.gmt_modified = Some(now);

    // update site
    TbSite::update_by_name(&mut *conn, &tb_site, &site.name).await?;

    let locations = TbLocation::select_by_site_id(&mut *conn, site_id).await?;
    let mut to_be_deleted = vec![];
    let mut to_be_added = vec![];
    let mut to_be_updated = vec![];

    let db_locations: HashMap<i64, &TbLocation> = locations
        .iter()
        .filter_map(|loc| loc.id.map(|id| (id, loc)))
        .collect();

    let mut used_ids = HashSet::new();

    for l in &site.locations {
        match l.id {
            Some(id) => {
                if let Some(db_loc) = db_locations.get(&(id as i64)) {
                    to_be_updated.push((l, *db_loc));
                    used_ids.insert(id as i64);
                }
            }
            None => {
                to_be_added.push(l);
            }
        }
    }

    to_be_deleted.extend(
        db_locations
            .keys()
            .filter(|&id| !used_ids.contains(id))
            .copied(),
    );

    // update location
    for (location, tb_location) in to_be_updated {
        let location_id = location.id.ok_or(anyhow::anyhow!("Id is empty"))? as i64;

        let mut new_location = convert_location(site_id, location, &now);
        new_location.gmt_create = tb_location.gmt_create;
        TbLocation::update_by_id(&mut *conn, &new_location, location_id).await?;

        let mut to_be_add: HashSet<(String, u16)> = location
            .upstream
            .iter()
            .filter(|item| item.contains(":"))
            .filter_map(|s| {
                let mut parts = s.split(':');
                let ip = parts.next()?.to_string();
                let port = parts.next()?.parse::<u16>().ok()?;
                Some((ip, port))
            })
            .collect();

        let backends = TbBackend::select_by_location_id(&mut *conn, location_id).await?;
        let mut to_be_delete = vec![];
        for b in &backends {
            let host = b.host.clone().ok_or(anyhow::anyhow!("Host is empty"))?;
            let port = b.port.ok_or(anyhow::anyhow!("Host is empty"))? as u16;
            let item = &(host, port);
            if !to_be_add.contains(item) {
                to_be_delete.push(b.id);
            } else {
                to_be_add.remove(item);
            }
        }

        for id in to_be_delete.into_iter().flatten() {
            TbBackend::delete_by_id(&mut *conn, id).await?;
        }

        for (host, port) in to_be_add {
            add_backend(&mut *conn, location_id, &host, port, &now).await?;
        }
    }
    // add location
    for location in to_be_added {
        let tb_location = convert_location(site_id, location, &now);
        let r = TbLocation::insert(&mut *conn, &tb_location).await?;
        let location_id = r.last_insert_id() as i64;
        for backend in &location.upstream {
            let addr: Vec<&str> = backend.split(":").collect();
            if addr.len() != 2 {
                continue;
            }
            add_backend(
                &mut *conn,
                location_id,
                addr[0].trim(),
                addr[1].trim().parse()?,
                &now,
            )
            .await?;
        }
    }
    // delete location
    for id in to_be_deleted {
        TbLocation::delete_by_id(&mut *conn, id).await?;
    }

    let (id, site) = find_site(&mut *conn, &site.name).await?;
    let s = serde_json::to_string_pretty(&site)?;
    let change_log = do_build_change_log(
        &mut *conn,
        site.cluster.clone(),
        LogType::Site,
        LogAction::Update,
        id,
        0,
        Some(s),
    )
    .await?;
    Ok((site, change_log))
}

async fn add_backend<'e, E: sqlx::MySqlExecutor<'e>>(
    rb: E,
    location_id: i64,
    host: &str,
    port: u16,
    now: &OffsetDateTime,
) -> anyhow::Result<()> {
    let tb_backend = TbBackend {
        id: None,
        location_id: Some(location_id),
        host: Some(host.to_string()),
        port: Some(port as i32),
        gmt_create: Some(*now),
        gmt_modified: Some(*now),
    };
    let _r = TbBackend::insert(rb, &tb_backend).await?;
    Ok(())
}

fn convert_location(site_id: i64, location: &ProxyLocation, now: &OffsetDateTime) -> TbLocation {
    TbLocation {
        id: None,
        site_id: Some(site_id),
        location: Some(location.path.as_str().to_owned()),
        proxy: Some(if location.proxy { 1 } else { 0 }),
        protocol: Some(location.protocol.to_string()),
        connection_timeout: Some(location.connection_timeout as i32),
        read_timeout: Some(location.read_timeout as i32),
        write_timeout: Some(location.write_timeout as i32),
        idle_timeout: Some(location.idle_timeout as i32),
        gmt_create: Some(*now),
        gmt_modified: Some(*now),
        sni: Some(location.sni.clone()),
        client_max_body_size: Some(location.client_max_body_size as i64),
        proxy_set_headers: location.proxy_set_headers.as_ref().and_then(|item| {
            if let Ok(headers) = convert_headers_to_string(item) {
                serde_json::to_string(&headers).ok()
            } else {
                None
            }
        }),
        proxy_add_headers: location.proxy_add_headers.as_ref().and_then(|item| {
            if let Ok(headers) = convert_headers_to_string(item) {
                serde_json::to_string(&headers).ok()
            } else {
                None
            }
        }),
        proxy_remove_headers: location.proxy_remove_headers.as_ref().and_then(|items| {
            let headers = items.iter().map(|h| h.as_str()).collect::<Vec<_>>();
            serde_json::to_string(&headers).ok()
        }),
        response_set_headers: location.response_set_headers.as_ref().and_then(|item| {
            if let Ok(headers) = convert_headers_to_string(item) {
                serde_json::to_string(&headers).ok()
            } else {
                None
            }
        }),
        response_add_headers: location.response_add_headers.as_ref().and_then(|item| {
            if let Ok(headers) = convert_headers_to_string(item) {
                serde_json::to_string(&headers).ok()
            } else {
                None
            }
        }),
        response_remove_headers: location.response_remove_headers.as_ref().and_then(|items| {
            let headers = items.iter().map(|h| h.as_str()).collect::<Vec<_>>();
            serde_json::to_string(&headers).ok()
        }),
        rewrite: location
            .rewrite
            .as_ref()
            .map(|(r, s)| "".to_owned() + r.as_str() + " " + s),
        http_version: location.http_version.as_ref().map(|v| v.to_string()),
        root_dir: location.root_dir.as_ref().map(|item| unsafe {
            String::from_utf8_unchecked(item.as_os_str().as_encoded_bytes().to_vec())
        }),
        auto_index: Some(if location.auto_index { 1 } else { 0 }),
    }
}
