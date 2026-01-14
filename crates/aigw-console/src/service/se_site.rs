use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, OnceLock},
};

use aigw_core::{
    ChangeLog, DynamicCert, HttpVersion, LogAction, LogType, ProxyLocation, Site, TlsPrivateKey,
    convert_headers, convert_headers_to_string, date_format_local, new_path_selector, new_rewrite,
};
use http::HeaderName;
use rbatis::{IPageRequest, RBatis, rbdc::DateTime};

use crate::{
    service::{Page, YYYY_MM_DD_FORMAT, do_build_change_log},
    storage::{
        tb_backend::TbBackend, tb_change_log::TbChangeLog, tb_location::TbLocation, tb_site::TbSite,
    },
};

pub async fn add_site(rb: &RBatis, site: &Site) -> anyhow::Result<(Site, ChangeLog)> {
    if site.locations.is_empty() {
        return Err(anyhow::anyhow!("Location is empty"));
    }

    for location in &site.locations {
        if location.proxy && location.upstream.is_empty() {
            let err = "Location '".to_string() + location.path.as_str() + "' backends is empty.";
            return Err(anyhow::anyhow!(err));
        }
    }
    let tx = rb.acquire_begin().await?;
    match do_add_new_site(&tx, site).await {
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

pub async fn modify_site(rb: &RBatis, site: &Site) -> anyhow::Result<(Site, ChangeLog)> {
    if site.locations.is_empty() {
        return Err(anyhow::anyhow!("Location is empty"));
    }

    for location in &site.locations {
        if location.proxy && location.upstream.is_empty() {
            let err = "Location '".to_string() + location.path.as_str() + "' backends is empty.";
            return Err(anyhow::anyhow!(err));
        }
    }
    let tx = rb.acquire_begin().await?;
    match do_modify_site(&tx, site).await {
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
    rb: &dyn rbatis::executor::Executor,
    name: &str,
    tls_cert: String,
    tls_private_key: String,
) -> anyhow::Result<(Site, ChangeLog)> {
    let r = TbSite::select_by_name(rb, name).await?;
    let mut tb_site = r.ok_or(anyhow::anyhow!("Site not found."))?;

    let now = DateTime::utc();

    let cert = DynamicCert::try_from(tls_cert.as_bytes())?;

    tb_site.tls_cert = Some(tls_cert);
    tb_site.tls_cert_start_date = Some(DateTime::from_timestamp(
        cert.cert.not_before().unix_timestamp(),
    ));
    tb_site.tls_cert_end_date = Some(DateTime::from_timestamp(
        cert.cert.not_after().unix_timestamp(),
    ));
    tb_site.tls_private_key = Some(tls_private_key);
    tb_site.gmt_modified = Some(now.clone());

    // update site
    TbSite::update_by_name(rb, &tb_site, name).await?;

    let (id, site) = find_site(rb, name).await?;
    let s = serde_json::to_string_pretty(&site)?;
    let change_log = do_build_change_log(
        rb,
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
    rb: &dyn rbatis::executor::Executor,
    name: &str,
) -> anyhow::Result<(u64, Site)> {
    let tb_site = TbSite::select_by_name(rb, name).await?;
    if let Some(server) = tb_site {
        let id: u64 = server.id.unwrap_or_default();
        let site = convert_tb_site(rb, server).await?;
        return Ok((id, site));
    }
    Err(anyhow::anyhow!("Resource not found"))
}

pub async fn build_change_log_delete_site(rb: &RBatis, name: &str) -> anyhow::Result<ChangeLog> {
    let (id, site) = find_site(rb, name).await?;
    let s = serde_json::to_string_pretty(&site)?;
    let tx = rb.acquire_begin().await?;

    TbChangeLog::delete_by_data_id(&tx, id).await?;
    // 1. delete site
    match TbSite::delete_by_name(&tx, name).await {
        Ok(r) => {
            if r.rows_affected < 1 {
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
        &tx,
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
    rb: &dyn rbatis::executor::Executor,
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
        id: tb_site.id,
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
        rate_limit: tb_site.rate_limit.map_or(0, |i| i),
        rate_limit_unit: tb_site
            .rate_limit_unit
            .map_or(1000, |i| if i == 0 { 1000 } else { i }),
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

    let locations = TbLocation::select_by_site_id(rb, site_id).await?;
    for location in locations {
        let location_id = location.id.ok_or(anyhow::anyhow!("Location Id is null"))?;
        let mut backends_array = vec![];
        let backends = TbBackend::select_by_location_id(rb, location_id).await?;
        for backend in backends {
            let b = backend.host.unwrap() + ":" + backend.port.unwrap().to_string().as_str();
            backends_array.push(b);
        }

        let p_location = ProxyLocation {
            id: Some(location_id),
            path: Arc::new(new_path_selector(&location.location.unwrap())?),
            proxy: location.proxy == 1,
            protocol: location.protocol.unwrap().as_str().try_into()?,
            lb: OnceLock::new(),
            upstream: backends_array,
            connection_timeout: location.connection_timeout.map_or(5, |t| t),
            read_timeout: location.read_timeout.map_or(5, |t| t),
            write_timeout: location.write_timeout.map_or(5, |t| t),
            idle_timeout: location.idle_timeout.map_or(60, |t| t),
            sni: location.sni.map_or("$host".to_string(), |item| item),
            client_max_body_size: location.client_max_body_size.map_or(0, |t| t),
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
            auto_index: location.auto_index == 1,
        };

        site.locations.push(Arc::new(p_location));
    }
    Ok(site)
}

pub async fn find_site_by_page(
    rb: &RBatis,
    page_request: &dyn IPageRequest,
    cluster_name: &str,
) -> anyhow::Result<Page<Site>> {
    let r = TbSite::select_page(rb, page_request, cluster_name).await?;
    let mut page = Page::new(r.page_no, r.page_size, r.total, vec![]);
    for tb_site in r.records {
        let site = convert_tb_site(rb, tb_site).await?;
        page.items.push(site);
    }
    Ok(page)
}

async fn do_add_new_site(
    rb: &dyn rbatis::executor::Executor,
    site: &Site,
) -> anyhow::Result<(Site, ChangeLog)> {
    let now = DateTime::utc();

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
                Some(DateTime::from_timestamp(
                    cert.cert.not_before().unix_timestamp(),
                )),
                Some(DateTime::from_timestamp(
                    cert.cert.not_after().unix_timestamp(),
                )),
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
        rate_limit: Some(site.rate_limit),
        rate_limit_unit: Some(site.rate_limit_unit),
        gmt_create: Some(now.clone()),
        gmt_modified: Some(now.clone()),
    };
    let r = TbSite::insert(rb, &tb_site).await?;
    let site_id = r
        .last_insert_id
        .as_u64()
        .ok_or(anyhow::anyhow!("Last insert id is null."))?;
    for location in &site.locations {
        let tb_location = convert_location(site_id, location, &now);
        let r = TbLocation::insert(rb, &tb_location).await?;
        let id = r.last_insert_id.as_u64();
        if let Some(location_id) = id {
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
                add_backend(rb, location_id, host, *port, &now).await?;
            }
        }
    }

    let (id, site) = find_site(rb, &site.name).await?;
    let s = serde_json::to_string_pretty(&site)?;
    let change_log = do_build_change_log(
        rb,
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
    rb: &dyn rbatis::executor::Executor,
    site: &Site,
) -> anyhow::Result<(Site, ChangeLog)> {
    let r = TbSite::select_by_name(rb, &site.name).await?;
    let mut tb_site = r.ok_or(anyhow::anyhow!("Server not found."))?;
    let site_id = tb_site.id.ok_or(anyhow::anyhow!("Server id not found."))?;
    let now = DateTime::utc();

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
                Some(DateTime::from_timestamp(
                    cert.cert.not_before().unix_timestamp(),
                )),
                Some(DateTime::from_timestamp(
                    cert.cert.not_after().unix_timestamp(),
                )),
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
    tb_site.rate_limit = Some(site.rate_limit);
    tb_site.rate_limit_unit = Some(site.rate_limit_unit);
    tb_site.tls_on = site.tls_on;
    tb_site.tls_enforce = site.tls_enforce;
    tb_site.acme_on = site.acme_on;
    tb_site.gmt_modified = Some(now.clone());

    // update site
    TbSite::update_by_name(rb, &tb_site, &site.name).await?;

    let locations = TbLocation::select_by_site_id(rb, site_id).await?;
    let mut to_be_deleted = vec![];
    let mut to_be_added = vec![];
    let mut to_be_updated = vec![];

    let db_locations: HashMap<u64, &TbLocation> = locations
        .iter()
        .filter_map(|loc| loc.id.map(|id| (id, loc)))
        .collect();

    let mut used_ids = HashSet::new();

    for l in &site.locations {
        match l.id {
            Some(id) => {
                if let Some(db_loc) = db_locations.get(&id) {
                    to_be_updated.push((l, *db_loc));
                    used_ids.insert(id);
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
        let location_id = location.id.ok_or(anyhow::anyhow!("Id is empty"))?;

        let mut new_location = convert_location(site_id, location, &now);
        new_location.gmt_create = tb_location.gmt_create.clone();
        TbLocation::update_by_id(rb, &new_location, location_id).await?;

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

        let backends = TbBackend::select_by_location_id(rb, location_id).await?;
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
            TbBackend::delete_by_id(rb, id).await?;
        }

        for (host, port) in to_be_add {
            add_backend(rb, location_id, &host, port, &now).await?;
        }
    }
    // add location
    for location in to_be_added {
        let tb_location = convert_location(site_id, location, &now);
        let r = TbLocation::insert(rb, &tb_location).await?;
        let id = r.last_insert_id.as_u64();
        if let Some(location_id) = id {
            for backend in &location.upstream {
                let addr: Vec<&str> = backend.split(":").collect();
                if addr.len() != 2 {
                    continue;
                }
                add_backend(
                    rb,
                    location_id,
                    addr[0].trim(),
                    addr[1].trim().parse()?,
                    &now,
                )
                .await?;
            }
        }
    }
    // delete location
    for id in to_be_deleted {
        TbLocation::delete_by_id(rb, id).await?;
    }

    let (id, site) = find_site(rb, &site.name).await?;
    let s = serde_json::to_string_pretty(&site)?;
    let change_log = do_build_change_log(
        rb,
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

async fn add_backend(
    rb: &dyn rbatis::executor::Executor,
    location_id: u64,
    host: &str,
    port: u16,
    now: &DateTime,
) -> anyhow::Result<()> {
    let tb_backend = TbBackend {
        id: None,
        location_id: Some(location_id),
        host: Some(host.to_string()),
        port: Some(port as u32),
        gmt_create: Some(now.clone()),
        gmt_modified: Some(now.clone()),
    };
    let _r = TbBackend::insert(rb, &tb_backend).await?;
    Ok(())
}

fn convert_location(site_id: u64, location: &ProxyLocation, now: &DateTime) -> TbLocation {
    TbLocation {
        id: None,
        site_id: Some(site_id),
        location: Some(location.path.as_str().to_owned()),
        proxy: if location.proxy { 1 } else { 0 },
        protocol: Some(location.protocol.to_string()),
        connection_timeout: Some(location.connection_timeout),
        read_timeout: Some(location.read_timeout),
        write_timeout: Some(location.write_timeout),
        idle_timeout: Some(location.idle_timeout),
        gmt_create: Some(now.clone()),
        gmt_modified: Some(now.clone()),
        sni: Some(location.sni.clone()),
        client_max_body_size: Some(location.client_max_body_size),
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
        auto_index: if location.auto_index { 1 } else { 0 },
    }
}
