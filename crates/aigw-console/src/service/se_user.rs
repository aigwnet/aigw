use std::time::Duration;

use rbatis::{RBatis, rbdc::DateTime};
use serde::{Deserialize, Serialize};

use crate::storage::{tb_session::TbSession, tb_user::TbUser};

#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub email: String,
    pub avatar: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UserPassword {
    pub password: String,
    pub new_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserExtInfo {
    pub acme_account: Option<String>,
}

pub async fn query_user(rb: &RBatis, user_or_email: &str) -> anyhow::Result<UserProfile> {
    let user = if user_or_email.contains('@') {
        TbUser::select_by_email(rb, user_or_email).await?
    } else {
        TbUser::select_by_name(rb, user_or_email).await?
    };

    if let Some(user) = user {
        let email = user.email.unwrap();
        let avatar = "https://dn-qiniu-avatar.qbox.me/avatar/".to_string()
            + hex::encode(md5::compute(email.clone()).0).as_str();
        return Ok(UserProfile {
            name: user.name.unwrap(),
            email,
            avatar: Some(avatar),
        });
    }
    Err(anyhow::anyhow!("User not found."))
}

pub async fn update_profile(rb: &RBatis, name: &str, profile: UserProfile) -> anyhow::Result<()> {
    TbUser::update_by_name(
        rb,
        &TbUser {
            id: None,
            name: Some(profile.name),
            email: Some(profile.email),
            real_name: None,
            ext_info: None,
            password: None,
            gmt_create: None,
            gmt_modified: Some(DateTime::utc()),
        },
        name,
    )
    .await?;
    Ok(())
}

pub async fn update_password(
    rb: &RBatis,
    name: &str,
    password: UserPassword,
) -> anyhow::Result<()> {
    TbUser::update_by_name(
        rb,
        &TbUser {
            id: None,
            name: None,
            email: None,
            password: Some(password.new_password),
            real_name: None,
            ext_info: None,
            gmt_create: None,
            gmt_modified: Some(DateTime::utc()),
        },
        name,
    )
    .await?;
    Ok(())
}

pub async fn update_ext_info(rb: &RBatis, email: &str, ext_info: String) -> anyhow::Result<()> {
    TbUser::update_by_email(
        rb,
        &TbUser {
            id: None,
            name: None,
            email: None,
            password: None,
            real_name: None,
            ext_info: Some(ext_info),
            gmt_create: None,
            gmt_modified: Some(DateTime::utc()),
        },
        email,
    )
    .await?;
    Ok(())
}

pub async fn check_password(
    rb: &RBatis,
    user_or_email: &str,
    password: &str,
) -> anyhow::Result<(bool, String, String, bool)> {
    let user = if user_or_email.contains('@') {
        TbUser::select_by_email(rb, user_or_email).await?
    } else {
        TbUser::select_by_name(rb, user_or_email).await?
    };

    if let Some(user) = user {
        let name = user.name.ok_or(anyhow::anyhow!("User name not found."))?;
        let email = user.email.ok_or(anyhow::anyhow!("User email not found."))?;
        let p = user
            .password
            .ok_or(anyhow::anyhow!("User password not found."))?;
        if p.eq(password) {
            let reset = p.eq(format!("{:x}", md5::compute(b"admin")).as_str())
                || email.eq("admin@test.test");
            return Ok((true, name, email, reset));
        }
        return Ok((false, name, email, false));
    }
    Err(anyhow::anyhow!("User not found."))
}

pub async fn login(
    rb: &RBatis,
    user_or_email: &str,
    password: &str,
    ip: &str,
    token: &str,
) -> anyhow::Result<(bool, bool)> {
    let (b, name, email, reset) = check_password(rb, user_or_email, password).await?;
    if b {
        let session = TbSession::select_by_token(rb, token).await?;
        if let Some(mut session) = session {
            session.gmt_modified = Some(DateTime::utc());
            TbSession::update_by_token(rb, &session, token).await?;
        } else {
            let now = DateTime::utc();
            TbSession::insert(
                rb,
                &TbSession {
                    id: None,
                    user: Some(name),
                    email: Some(email),
                    login_ip: Some(ip.to_owned()),
                    token: Some(token.to_owned()),
                    gmt_create: Some(now.clone()),
                    gmt_modified: Some(now),
                },
            )
            .await?;
        }
        Ok((true, reset))
    } else {
        Ok((false, reset))
    }
}

pub async fn token_validate(
    rb: &RBatis,
    token: &str,
) -> anyhow::Result<(bool, Option<String>, Option<String>)> {
    let session = TbSession::select_by_token(rb, token).await?;
    if let Some(mut session) = session
        && let Some(gmt_modified) = session.gmt_modified
        && gmt_modified.add(Duration::from_secs(1800)) > DateTime::utc()
    {
        session.gmt_modified = Some(DateTime::utc());
        TbSession::update_by_token(rb, &session, token).await?;
        return Ok((true, session.user, session.email));
    }

    Ok((false, None, None))
}

pub async fn find_default_user(rb: &RBatis) -> anyhow::Result<Option<(String, String)>> {
    let user = TbUser::select_default_user(rb).await?;
    let r = user.map(|user| (user.name.unwrap(), user.email.unwrap()));
    Ok(r)
}
