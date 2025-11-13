use serde::{Deserialize, Deserializer, Serializer};
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_monitor;
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_monitor_cluster;
pub(crate) mod tb_analytics_monitor_cluster_hour;
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_traffic;
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_traffic_cluster;
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_analytics_traffic_cluster_hour;
pub(crate) mod tb_backend;
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_change_log;
pub(crate) mod tb_cluster;
#[allow(clippy::too_many_arguments)]
pub(crate) mod tb_cluster_ip_cidr;
pub(crate) mod tb_console;
pub(crate) mod tb_location;
pub(crate) mod tb_lock;
pub(crate) mod tb_server;
pub(crate) mod tb_session;
pub(crate) mod tb_site;
pub(crate) mod tb_task;
pub(crate) mod tb_user;

fn from_i8_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let i = i8::deserialize(deserializer)?;
    Ok(i != 0)
}

fn serialize_bool_to_i8<S>(b: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i8(if *b { 1 } else { 0 })
}
