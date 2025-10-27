use aigw_core::{ChangeLog, Site};
use anyhow::anyhow;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use log::{debug, error};
use rbatis::{PageRequest, RBatis};
use tokio::sync::mpsc::Sender;

use crate::{
    server::http::{
        ApiContext, ApiData, ApiError, ApiResponseResult, Pagination, auth::ExtractUser,
    },
    service::{
        Page, add_site, apply_cert, asn1time_to_datetime, build_change_log_delete_site, find_site,
        find_site_by_page, modify_site, update_cert,
    },
};

pub(crate) struct HttpApiSite {}

impl HttpApiSite {
    pub async fn add(
        ExtractUser(_user, email): ExtractUser,
        State(context): State<ApiContext>,
        Json(site): Json<Site>,
    ) -> ApiResponseResult<Site> {
        let (site, change_log) = add_site(&context.database_client.rb, &site)
            .await
            .map_err(ApiError::from)?;
        let _ = context.sender.send(change_log).await;

        let acme_on = site.acme_on;
        if acme_on {
            let rb = context.database_client.rb.clone();
            let sender = context.sender.clone();
            let cluster = site.cluster.clone();
            let name = site.name.clone();
            let alt_names = site.alt_names.clone();

            tokio::spawn(async move {
                let r =
                    HttpApiSite::add_cert_and_notify(rb, sender, cluster, name, alt_names, email)
                        .await;
                if let Err(r) = r {
                    error!("{:?}", r);
                }
            });
        }
        Ok(ApiData(Some(site)))
    }

    async fn add_cert_and_notify(
        rb: RBatis,
        sender: Sender<ChangeLog>,
        cluster: String,
        name: String,
        alt_names: Vec<String>,
        email: Option<String>,
    ) -> anyhow::Result<()> {
        let email = email.map_or(Err(anyhow!("User not found")), |u| Ok(u))?;

        let mut domains = vec![name.as_str()];
        for s in &alt_names {
            domains.push(s.as_str());
        }

        let cert = apply_cert(&rb, &sender, cluster.clone(), &email, &domains[..]).await?;

        let rx = rb.acquire_begin().await?;
        match update_cert(&rx, &name, cert.tls_cert, cert.tls_private_key).await {
            Ok((_, change_log)) => {
                rx.commit().await?;
                let _ = sender.send(change_log).await;
            }
            Err(_) => {
                rx.rollback().await?;
            }
        }

        Ok(())
    }

    pub async fn update(
        ExtractUser(_user, email): ExtractUser,
        Path(name): Path<String>,
        State(context): State<ApiContext>,
        Json(site): Json<Site>,
    ) -> ApiResponseResult<Site> {
        let (_, old_site) = find_site(&context.database_client.rb, name.as_str()).await?;

        let (site, change_log) = modify_site(&context.database_client.rb, &site)
            .await
            .map_err(ApiError::from)?;
        let _ = context.sender.send(change_log).await;

        let update_cert = !old_site
            .alt_names
            .clone()
            .sort()
            .eq(&site.alt_names.clone().sort())
            || old_site.tls_cert.map_or(true, |cert| {
                asn1time_to_datetime(cert.cert.not_before())
                    .ok()
                    .map_or(true, |d| {
                        debug!(
                            "{} {} {}",
                            chrono::Utc::now().timestamp(),
                            d.timestamp(),
                            chrono::Utc::now().timestamp() - d.timestamp()
                        );
                        chrono::Utc::now().timestamp() - d.timestamp() > 30 * 24 * 3600
                    })
            });

        if site.tls_on && site.acme_on && update_cert {
            let rb = context.database_client.rb.clone();
            let sender = context.sender.clone();
            let cluster = site.cluster.clone();
            let name = site.name.clone();
            let alt_names = site.alt_names.clone();

            tokio::spawn(async move {
                let r = HttpApiSite::update_cert_and_notify(
                    rb, sender, cluster, name, alt_names, email,
                )
                .await;
                if let Err(r) = r {
                    error!("{:?}", r);
                }
            });
        }
        Ok(ApiData(Some(site)))
    }

    async fn update_cert_and_notify(
        rb: RBatis,
        sender: Sender<ChangeLog>,
        cluster: String,
        name: String,
        alt_names: Vec<String>,
        email: Option<String>,
    ) -> anyhow::Result<()> {
        let email = email.map_or(Err(anyhow!("User not found")), |u| Ok(u))?;

        let mut domains = vec![name.as_str()];
        for s in &alt_names {
            domains.push(s.as_str());
        }

        let cert = apply_cert(&rb, &sender, cluster.clone(), &email, &domains[..]).await?;

        let rx = rb.acquire_begin().await?;
        match update_cert(&rx, &name, cert.tls_cert, cert.tls_private_key).await {
            Ok((_, change_log)) => {
                rx.commit().await?;
                let _ = sender.send(change_log).await;
            }
            Err(_) => {
                rx.rollback().await?;
            }
        }

        Ok(())
    }

    pub async fn query(
        Path(name): Path<String>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Site> {
        let (_, site) = find_site(&context.database_client.rb, name.as_str())
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(site)))
    }

    pub async fn query_by_page(
        Path(cluster_name): Path<String>,
        Query(page): Query<Pagination>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<Page<Site>> {
        let mut page_request = PageRequest::new(page.page, page.page_size);
        page_request = page_request.set_do_count(true);
        let r = find_site_by_page(&context.database_client.rb, &page_request, &cluster_name)
            .await
            .map_err(ApiError::from)?;
        Ok(ApiData(Some(r)))
    }

    pub async fn delete(
        Path(name): Path<String>,
        State(context): State<ApiContext>,
    ) -> ApiResponseResult<bool> {
        let change_log = build_change_log_delete_site(&context.database_client.rb, name.as_str())
            .await
            .map_err(ApiError::from)?;
        let _ = context.sender.send(change_log).await;
        Ok(ApiData(None))
    }
}
