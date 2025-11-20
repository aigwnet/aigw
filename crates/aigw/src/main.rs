use core::panic;
use std::{fs, sync::Arc};
mod server;
mod version {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}
use aigw_core::init_logger;
use dashmap::DashMap;
use server::{AigwConfig, ServerOpt, Storage};

use crate::server::GeoLite;

pub(crate) static SERVER: &str = "Aigw";

lazy_static::lazy_static! {}

fn main() -> anyhow::Result<()> {
    let mut args = ServerOpt::parse_args();

    #[cfg(unix)]
    if args.daemon {
        use aigw_core::daemonize;

        daemonize(
            args.user.as_ref(),
            args.group.as_ref(),
            args.pid_file.as_ref().map_or("/tmp/aigw.pid", |s| s),
        );
        args.daemon = false;
    }

    let targets = DashMap::new();
    targets.insert("console", "console");
    targets.insert("access", "access");
    targets.insert("test", "test");
    init_logger(args.log_dir.as_ref().map_or("logs", |s| s), targets, &[]);

    let config_file = if let Some(config_file) = args.config.as_ref() {
        config_file
    } else {
        "conf/aigw.toml"
    };

    let config = fs::read_to_string(config_file)?;
    let config: AigwConfig = toml::from_str(config.as_str())?;

    let storage = Arc::new(Storage::new(
        config.basic().data_dir().as_ref(),
        config.console().cluster().to_owned(),
    )?);
    storage.load_cluster()?;
    storage.load_sites()?;

    let geo_lite_file = if let Some(geo_lite_file) = args.geo_lite.as_ref() {
        geo_lite_file
    } else {
        "crates/aigw/assets/GeoLite2-Country.mmdb"
    };

    let get_lite = Arc::new(GeoLite::new(geo_lite_file)?);
    if let Err(e) = server::run(args, Arc::new(config), storage, get_lite) {
        panic!("Start server failed. {}", e)
    }

    Ok(())
}
