use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    sync::Arc,
};

use aigw_core::ChangeLog;
use conf::AigwConsoleConfig;
use log::{Level, LevelFilter, debug, info};
use regex::Regex;
use simplelog::{
    Color, ColorChoice, CombinedLogger, ConfigBuilder, SharedLogger, TermLogger, TerminalMode,
    WriteLogger,
};
use storage::db::DatabaseClient;
use tokio::{runtime::Runtime, sync::Mutex};

use crate::args::AigwConsoleArgs;

mod args;
mod conf;
mod server;
mod service;
mod storage;

fn main() -> anyhow::Result<()> {
    let args = AigwConsoleArgs::do_parse();

    if args.daemon {
        let mut daemonize = daemonize::Daemonize::new();
        if let Some(user) = &args.user {
            daemonize = daemonize.user(&user as &str);
        }
        if let Some(group) = &args.group {
            daemonize = daemonize.group(&group as &str);
        }
        if let Some(pid_file) = &args.pid_file {
            daemonize = daemonize.pid_file(pid_file).chown_pid_file(true);
        }
        daemonize.start()?;
    }

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
        let logger = WriteLogger::new(LevelFilter::Info, log_config, log_file);
        loggers.push(logger);
    }

    CombinedLogger::init(loggers)?;

    let config_file = if let Some(config_file) = args.config.as_ref() {
        config_file
    } else {
        "conf/aigwc.toml"
    };

    let config = fs::read_to_string(config_file)?;
    let mut config: AigwConsoleConfig = toml::from_str(config.as_str())?;
    if let Some(ui) = &args.ui {
        config.server.ui = Some(ui.clone());
    }

    let json = serde_json::to_string_pretty(&config)?;
    let re = Regex::new(r#""password"\s*:\s*("[^"]*)""#).expect("Invalid regex");

    let s = re
        .replace_all(&json, |caps: &regex::Captures| {
            let value = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            let len = value.trim_start_matches('"').trim_end_matches('"').len();
            format!(r#""password": "{}""#, "*".repeat(len))
        })
        .to_string();
    debug!("{}", s);

    let config = Arc::new(config);
    let database_client = DatabaseClient::default();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .max_blocking_threads(20)
        .enable_all()
        .build()?;
    let rt = Arc::new(rt);

    // init mysql database
    rt.block_on(async {
        database_client
            .init(
                &config.database.url,
                Some(&config.database.user),
                Some(&config.database.password),
            )
            .await
    })?;
    let database_client = Arc::new(database_client);

    if args.install {
        install(rt, database_client)
    } else {
        start(rt, database_client, config)
    }
}

fn install(rt: Arc<Runtime>, database_client: Arc<DatabaseClient>) -> anyhow::Result<()> {
    rt.block_on(async { database_client.install().await })?;

    Ok(())
}

fn start(
    rt: Arc<Runtime>,
    database_client: Arc<DatabaseClient>,
    config: Arc<AigwConsoleConfig>,
) -> anyhow::Result<()> {
    let connections = Arc::new(Mutex::new(HashMap::new()));

    let (sender, receiver) = tokio::sync::mpsc::channel::<ChangeLog>(1024);

    // start http server
    let database_client_for_http = database_client.clone();
    let config_for_http = config.clone();
    let sender_for_http = sender.clone();
    rt.spawn(async move {
        server::http::run(sender_for_http, database_client_for_http, config_for_http).await
    });

    // start to broadcast changelog to other aigw console servers.
    let database_client_for_broadcast = database_client.clone();
    rt.spawn(
        async move { server::broadcast::broadcast(database_client_for_broadcast, receiver).await },
    );

    // start broadcast server
    let connections_for_broadcast = connections.clone();
    let config_for_broadcast = config.clone();
    let database_client_for_broadcast = database_client.clone();
    rt.spawn(async move {
        info!(
            "Start broadcast: {}",
            config_for_broadcast.server.boradcast.port
        );
        server::broadcast::run(
            database_client_for_broadcast,
            connections_for_broadcast,
            config_for_broadcast.server.boradcast.port,
        )
        .await
    });

    let database_client_for_cert = database_client.clone();
    rt.spawn(async move {
        service::renew_certs(database_client_for_cert, sender).await;
    });

    let database_client_for_analytics_minute = database_client.clone();
    rt.spawn(async move {
        service::start_analytics_minute(database_client_for_analytics_minute).await;
    });

    let database_client_for_analytics_hour = database_client.clone();
    rt.spawn(async move {
        service::start_analytics_hour(database_client_for_analytics_hour).await;
    });

    // start tcp server
    rt.block_on(async {
        let shutdown = tokio::signal::ctrl_c();
        server::tcp::run(
            connections,
            database_client,
            config.server.tcp.port,
            config.server.tcp.max_connections,
            &config.server.key,
            shutdown,
        )
        .await;
    });

    Ok(())
}
