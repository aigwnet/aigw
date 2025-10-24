use core::panic;
use std::{
    fs::{self, OpenOptions},
    sync::Arc,
};

mod server;
mod version {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

use server::{AigwConfig, ServerOpt, Storage};
use simplelog::{
    Color, ColorChoice, CombinedLogger, ConfigBuilder, Level, LevelFilter, SharedLogger,
    TermLogger, TerminalMode, WriteLogger,
};

use crate::server::GeoLite;

pub(crate) static SERVER: &str = "Aigw";

lazy_static::lazy_static! {}

fn main() -> anyhow::Result<()> {
    let args = ServerOpt::parse_args();

    let mut builder = ConfigBuilder::new();
    let r = builder
        .set_level_color(Level::Error, Some(Color::Magenta))
        .set_level_color(Level::Trace, Some(Color::Green))
        .set_time_offset_to_local();
    let log_config = match r {
        Ok(builder) => builder.build(),
        Err(builder) => builder.build(),
    };

    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![];
    loggers.push(TermLogger::new(
        LevelFilter::Info,
        log_config.clone(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ));

    if let Some(file) = &args.log_file {
        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(file)?;
        let logger = WriteLogger::new(LevelFilter::Trace, log_config, log_file);
        loggers.push(logger);
    }

    CombinedLogger::init(loggers)?;

    let config_file = if let Some(config_file) = args.config.as_ref() {
        config_file
    } else {
        "conf/aigw.toml"
    };

    let config = fs::read_to_string(config_file)?;
    let config: AigwConfig = toml::from_str(config.as_str())?;

    let storage = Arc::new(Storage::new(config.basic().data_dir().as_ref())?);
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
