use std::{sync::Arc, time::Duration};

use crate::{
    service::{
        Account, AccountBuilder, AuthorizationStatus, Certificate, ChallengeStatus,
        DirectoryBuilder, OrderBuilder, OrderStatus,
        se_changlog::do_build_change_log,
        se_lock,
        se_user::{self, UserExtInfo, find_default_user},
        update_cert,
    },
    storage::{db::DatabaseClient, tb_site::TbSite, tb_user::TbUser},
};
use aigw_core::{AcmeToken, ChangeLog, LOCAL_IP, LogAction, LogType, TlsPrivateKey};
use anyhow::anyhow;
use base64::{Engine, prelude::BASE64_STANDARD};
use rbatis::{PageRequest, RBatis};
use rcgen::{KeyPair, PKCS_RSA_SHA512};
use tokio::{sync::mpsc::Sender, time::interval};
use tracing::{debug, error, info};

const LETS_ENCRYPT_URL: &str = "https://acme-v02.api.letsencrypt.org/directory";
// const LETS_ENCRYPT_URL: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";

/// Generate a new RSA Private key
fn gen_rsa_private_key() -> anyhow::Result<TlsPrivateKey> {
    let key_pair = KeyPair::generate_for(&PKCS_RSA_SHA512)?;
    let s = &key_pair.serialize_pem();
    let key = TlsPrivateKey::try_from(s.as_bytes())?;
    Ok(key)
}

async fn get_account(rb: &RBatis, email: &str) -> anyhow::Result<Account> {
    let user = TbUser::select_by_email(rb, email)
        .await?
        .ok_or(anyhow!("User not found"))?;

    if let Some(ext_info) = &user.ext_info {
        let ext: UserExtInfo = serde_json::from_str(ext_info)?;
        if let Some(acount) = ext.acme_account {
            //
            let acount = BASE64_STANDARD.decode(acount)?;
            let acount = String::from_utf8_lossy(&acount);
            let account: Account = serde_json::from_str(&acount)?;
            return Ok(account);
        }
    }
    Err(anyhow!("Account not exist"))
}

async fn save_account(rb: &RBatis, email: &str, account: &Account) -> anyhow::Result<()> {
    let account = serde_json::to_string_pretty(&account)?;
    let account = BASE64_STANDARD.encode(account);
    let ext = UserExtInfo {
        acme_account: Some(account),
    };
    se_user::update_ext_info(rb, email, serde_json::to_string_pretty(&ext)?).await?;
    Ok(())
}

pub async fn apply_cert(
    rb: &RBatis,
    sender: &Sender<ChangeLog>,
    cluster: String,
    email: &str,
    domains: &[&str],
) -> anyhow::Result<Certificate> {
    info!(target:"certificate", "Start to apply cert: {}, {:?}", email, domains);
    if domains.is_empty() {
        return Err(anyhow!("Domain is blank."));
    }

    let dir = DirectoryBuilder::new(LETS_ENCRYPT_URL.to_string())
        .build()
        .await?;

    debug!(
        target:"certificate", "Directory: {:?}, {:?}",
        dir.revoke_cert_url, dir.key_change_url
    );

    let r = get_account(rb, email).await;
    let account = if let Ok(mut account) = r {
        account.directory = Some(dir);
        Arc::new(account)
    } else {
        debug!("Get account error: {:?}", r.err());
        let contact = "mailto:".to_string() + email;

        // Create an ACME account to use for the order. For production
        // purposes, you should keep the account (and private key), so
        // you can renew your certificate easily.
        let mut builder = AccountBuilder::new(dir.clone());
        builder.contact(vec![contact]);
        builder.terms_of_service_agreed(true);
        builder.only_return_existing(false);
        let account = builder.build().await?;
        let _ = save_account(rb, email, &account).await;
        account
    };

    // Create a new order for a specific domain name.
    let mut builder = OrderBuilder::new(account);
    for domain in domains {
        builder.add_dns_identifier(domain.to_string());
    }
    let order = builder.build().await?;
    debug!(
        target:"certificate", "Build Order: {:?}, {:?}, {:?},error: {:?}",
        order.not_after, order.not_before, order.expires, order.error
    );

    // Get the list of needed authorizations for this order.
    let authorizations = order.authorizations().await?;

    for auth in authorizations {
        // Get an http-01 challenge for this authorization (or panic
        // if it doesn't exist).
        let challenge = auth.get_challenge("http-01").unwrap();

        debug!(
            target:"certificate", "Auth: {:?}, {:?}, {:?}, Challenge: {:?}, {:?} {:?}",
            auth.identifier,
            auth.expires,
            auth.wildcard,
            challenge.validated,
            challenge.token,
            challenge.key_authorization()?
        );

        let data = serde_json::to_string_pretty(&AcmeToken {
            host: auth.identifier.value.clone(),
            token: challenge.token.clone().unwrap(),
            proof: challenge.key_authorization()?.clone().unwrap(),
        })?;
        let change_log = do_build_change_log(
            rb,
            cluster.clone(),
            LogType::Acme,
            LogAction::Add,
            chrono::Utc::now().timestamp() as u64,
            300,
            Some(data),
        )
        .await?;

        sender.send(change_log).await?;

        tokio::time::sleep(Duration::from_secs(10)).await;

        // At this point in time, you must configure your webserver to serve
        // a file at `https://example.com/.well-known/${challenge.token}`
        // with the content of `challenge.key_authorization()??`.

        // Start the validation of the challenge.
        let challenge = challenge.validate().await?;

        // Poll the challenge every 5 seconds until it is in either the
        // `valid` or `invalid` state.
        let challenge = challenge.wait_done(Duration::from_secs(5), 6).await?;
        if let Some(err) = challenge.error {
            error!("{:?}", err);
            return Err(anyhow::anyhow!("error."));
        }

        assert_eq!(challenge.status, ChallengeStatus::Valid);

        // You can now remove the challenge file hosted on your webserver.

        // Poll the authorization every 5 seconds until it is in either the
        // `valid` or `invalid` state.
        let authorization = auth.wait_done(Duration::from_secs(5), 3).await?;
        assert_eq!(authorization.status, AuthorizationStatus::Valid)
    }

    // Poll the order every 5 seconds until it is in either the
    // `ready` or `invalid` state. Ready means that it is now ready
    // for finalization (certificate creation).
    let order = order.wait_ready(Duration::from_secs(5), 3).await?;

    assert_eq!(order.status, OrderStatus::Ready);

    // Generate a Private key for the certificate.
    let pkey = gen_rsa_private_key()?;

    // Create a certificate signing request for the order, and request
    // the certificate.
    let order = order.finalize(&pkey).await?;

    // Poll the order every 5 seconds until it is in either the
    // `valid` or `invalid` state. Valid means that the certificate
    // has been provisioned, and is now ready for download.
    let order = order.wait_done(Duration::from_secs(5), 3).await?;
    debug!(
        target:"certificate", "Order Done: {:?}, {:?}, {:?},error: {:?}",
        order.not_after, order.not_before, order.expires, order.error
    );
    assert_eq!(order.status, OrderStatus::Valid);

    // Download the certificate, and panic if it doesn't exist.
    let cert = order.certificate().await?;

    info!(target:"certificate", "Apply cert: {}, {:?} successfully!", email, domains);

    let pkey = pkey.try_to_string()?;

    Ok(Certificate {
        tls_private_key: pkey,
        tls_cert: cert,
    })
}

pub async fn renew_certs(database_client: Arc<DatabaseClient>, sender: Sender<ChangeLog>) {
    let mut interval = interval(Duration::from_secs(24 * 3600));
    loop {
        interval.tick().await;
        let lock_key = "acme".to_string();

        let host = &LOCAL_IP;
        let r = se_lock::try_acquire_lock(&database_client.rb, &lock_key, host, 3600).await;
        if r {
            info!(
                target:"certificate", "Check domains with certificates about to expire and initiate the renewal process."
            );
            let default_user = find_default_user(&database_client.rb).await;
            if let Ok(Some((_, email))) = &default_user {
                let r = do_renew_certs(&database_client.rb, &sender, email).await;
                if let Err(e) = r {
                    error!("Renew certs error: {:?}", e);
                }
            }
        }
        se_lock::release_lock(&database_client.rb, &lock_key).await;
    }
}

async fn do_renew_certs(
    rb: &RBatis,
    sender: &Sender<ChangeLog>,
    email: &str,
) -> anyhow::Result<()> {
    let mut page_no = 1;

    loop {
        let page_request = PageRequest::new(page_no, 20);
        let r = TbSite::select_acme_cert_about_to_expire(rb, &page_request).await?;
        if r.records.is_empty() {
            return Ok(());
        }
        for item in r.records {
            let cluster = item
                .cluster_name
                .ok_or(anyhow::anyhow!("Cluster is empty"))?;
            let name = item.name.ok_or(anyhow::anyhow!("Domain is empty"))?;
            info!(target:"certificate", "Start renewal domain {} certificate.", name);
            let alt_names = item
                .alt_names
                .as_ref()
                .map_or("", |s| s)
                .split(",")
                .collect::<Vec<&str>>();
            let mut domains = vec![name.as_str()];
            for item in alt_names {
                if item.is_empty() {
                    continue;
                }
                domains.push(item);
            }

            // apply new cert
            let cert = apply_cert(rb, sender, cluster, email, &domains).await?;
            // update cert
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

            info!(target:"certificate", "Renewal domain {} certificate successfully.", name);
        }

        page_no += 1;
    }
}
