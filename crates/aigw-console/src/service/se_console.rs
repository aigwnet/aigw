use std::time::Duration;

use aigw_core::ChangeLog;
use bytes::BytesMut;
use log::{debug, error};
use rbatis::{PageRequest, rbdc::DateTime};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::storage::tb_console::TbConsole;

pub async fn update_or_insert_local_peer(
    rb: &rbatis::RBatis,
    host: &str,
    port: u16,
) -> anyhow::Result<()> {
    let now = DateTime::utc();
    let item = TbConsole::select_by_host_port(rb, host, port).await?;
    // update last_active_time
    if let Some(mut item) = item {
        item.last_active_time = Some(now.clone());
        item.gmt_modified = Some(now);
        let _r = TbConsole::update_by_id(rb, &item, item.id.unwrap()).await;
    }
    // insert new item
    else {
        let item = TbConsole {
            id: None,
            host: Some(host.to_string()),
            port: Some(port),
            last_active_time: Some(now.clone()),
            gmt_create: Some(now.clone()),
            gmt_modified: Some(now),
        };
        let _ = TbConsole::insert(rb, &item).await?;
    }

    Ok(())
}

pub async fn send_change_log_to_peers(
    rb: &rbatis::RBatis,
    changelog: ChangeLog,
) -> anyhow::Result<()> {
    let changelog = &changelog.to_vec();
    // When a changelog is received, perform a paged query on the aigw console server,
    // and distribute the content to all aigw console servers, including itself.
    let mut page_no = 1;
    let mut page_request = PageRequest::default();
    page_request = page_request.set_page_size(20);
    loop {
        let r = select_console_peer_by_page(rb, &page_request).await;
        match r {
            Ok(records) => {
                if records.is_empty() {
                    break;
                }

                for item in records {
                    //
                    if let Err(e) = send_change_log_to_peer(
                        item.host.unwrap().as_str(),
                        item.port.unwrap(),
                        changelog,
                    )
                    .await
                    {
                        error!("Send change log to aigw console server error, {:?}", e);
                    }
                }

                page_no += 1;
                page_request = page_request.set_page_no(page_no);
            }
            Err(e) => {
                error!("Query error. {:?}", e);
            }
        }
    }

    Ok(())
}

async fn send_change_log_to_peer(host: &str, port: u16, data: &[u8]) -> anyhow::Result<()> {
    let addr = host.to_string() + ":" + port.to_string().as_str();
    let mut stream = TcpStream::connect(addr).await?;
    debug!("Send changelog to other server: {}", data.len());
    let length = data.len() as u32;
    stream.write_u32(length).await?;
    stream.write_all(data).await?;
    stream.flush().await?;

    let length = stream.read_u32().await?;
    if length > 0 {
        let mut buf = BytesMut::with_capacity(64);
        unsafe {
            buf.set_len(length as usize);
        }
        let _ = stream.read(&mut buf).await?;
    }

    Ok(())
}

async fn select_console_peer_by_page(
    rb: &rbatis::RBatis,
    page_request: &PageRequest,
) -> anyhow::Result<Vec<TbConsole>> {
    let page = TbConsole::select_by_page(rb, page_request).await?;
    let mut r = vec![];
    for item in page.records {
        // If last_active_time has not been updated for more than 60 seconds, the node is considered unreachable
        if DateTime::utc() - item.last_active_time.clone().unwrap() > Duration::from_millis(60000) {
            continue;
        }
        r.push(item);
    }
    Ok(r)
}
