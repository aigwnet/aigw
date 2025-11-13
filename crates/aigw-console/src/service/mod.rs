mod acme;
mod map;
mod se_acme;
mod se_analytics;
mod se_changlog;
mod se_cluster;
mod se_cluster_ip_cidr;
mod se_console;
mod se_lock;
mod se_server;
mod se_site;
mod se_task;
mod se_user;

pub(crate) use acme::*;
pub(crate) use se_acme::apply_cert;
pub(crate) use se_acme::renew_certs;
pub(crate) use se_analytics::AnalyticsMonitorItem;
pub(crate) use se_analytics::AnalyticsTrafficItem;
pub(crate) use se_analytics::ExtInfo;
pub(crate) use se_analytics::get_analytics_monitor;
pub(crate) use se_analytics::get_analytics_monitor_server;
pub(crate) use se_analytics::get_analytics_traffic;
pub(crate) use se_analytics::get_analytics_traffic_1day;
pub(crate) use se_analytics::get_analytics_traffic_1month;
pub(crate) use se_analytics::get_analytics_traffic_ext_info_1month;
pub(crate) use se_analytics::save_ping;
pub(crate) use se_analytics::start_analytics_hour;
pub(crate) use se_analytics::start_analytics_minute;
pub(crate) use se_changlog::do_build_change_log;
pub(crate) use se_changlog::send_all_sites_to_aigw;
pub(crate) use se_changlog::send_change_logs_to_aigw;
pub(crate) use se_cluster::add_new_cluster;
pub(crate) use se_cluster::delete_cluster;
pub(crate) use se_cluster::find_all;
pub(crate) use se_cluster::find_cluster;
pub(crate) use se_cluster::find_cluster_by_name;
pub(crate) use se_cluster::find_cluster_by_page;
pub(crate) use se_cluster::modify_cluster;
pub(crate) use se_cluster_ip_cidr::ClusterIpCidr;
pub(crate) use se_cluster_ip_cidr::find_ip_cidr_by_page;
pub(crate) use se_console::send_change_log_to_peers;
pub(crate) use se_console::update_or_insert_local_peer;
pub(crate) use se_server::Server;
pub(crate) use se_server::find_server_by_page;
pub(crate) use se_server::update_or_insert_server;
pub(crate) use se_site::add_site;
pub(crate) use se_site::asn1time_to_datetime;
pub(crate) use se_site::build_change_log_delete_site;
pub(crate) use se_site::find_site;
pub(crate) use se_site::find_site_by_page;
pub(crate) use se_site::modify_site;
pub(crate) use se_site::update_cert;
pub(crate) use se_user::UserPassword;
pub(crate) use se_user::UserProfile;
pub(crate) use se_user::check_password;
pub(crate) use se_user::login;
pub(crate) use se_user::query_user;
pub(crate) use se_user::token_validate;
pub(crate) use se_user::update_password;
pub(crate) use se_user::update_profile;

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Page<T: Send + Sync> {
    /// data
    pub items: Vec<T>,
    /// total num
    pub total: u64,
    /// current page index
    pub page_no: u64,
    /// default 10
    pub page_size: u64,
    /// Control whether to execute count statements to count the total number
    pub do_count: bool,
}

impl<T: Send + Sync> Page<T> {
    pub fn new(page_no: u64, mut page_size: u64, total: u64, items: Vec<T>) -> Self {
        if page_size == 0 {
            page_size = rbatis::DEFAULT_PAGE_SIZE;
        }
        if page_no < 1 {
            return Self {
                total,
                page_size,
                page_no: 1u64,
                items,
                do_count: true,
            };
        }
        Self {
            total,
            page_size,
            page_no,
            items,
            do_count: true,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Certificate {
    pub tls_private_key: String,
    pub tls_cert: String,
}
