mod acme;
mod console;
#[cfg(target_os = "linux")]
mod epbf;
mod runtime;
mod shutdown;
mod storage;

use std::sync::Arc;

use pingora_core::{
    apps::HttpServerOptions,
    listeners::tls::TlsSettings,
    server::{Server, configuration::ServerConf},
    tls::ssl::SslVersion,
};
use pingora_limits::rate::Rate;
use pingora_proxy::http_proxy_service_with_name;
pub(crate) use runtime::{AigwConfig, GeoLite, ServerOpt};
use runtime::{AigwProxy, DynamicTlsAccept};
use shutdown::run_args;
pub(crate) use storage::Storage;
use tokio::sync::broadcast;
use tracing::info;

use crate::server::console::AigwConsoleService;
pub struct RateLimit {
    pub(crate) max_request: isize,
    pub(crate) rate: Rate,
}

pub fn run(
    args: ServerOpt,
    config: Arc<AigwConfig>,
    storage: Arc<Storage>,
    geo_lite: Arc<GeoLite>,
) -> anyhow::Result<()> {
    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let shutdown_tx = Arc::new(shutdown_tx);

    #[cfg(target_os = "linux")]
    let ebpf_handler = Arc::new({
        let rt = tokio::runtime::Builder::new_current_thread().build()?;
        rt.block_on(async { epbf::run(config.basic().iface(), args.ebpf.as_ref()) })
    }?);

    let aigwc_service = AigwConsoleService::new(
        storage.clone(),
        shutdown_tx.clone(),
        config.console().address(),
        config.console().key(),
        config.console().cluster().to_string(),
        #[cfg(target_os = "linux")]
        ebpf_handler,
    );

    let mut works = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(32);
    if works < 8 {
        works = 8;
    }
    let main_conf = ServerConf {
        grace_period_seconds: Some(30),
        pid_file: args
            .pid_file
            .as_ref()
            .map_or("/tmp/aigw.pid".to_string(), |p| p.clone()),
        upgrade_sock: "/tmp/aigw_upgrade.sock".to_string(),
        user: args.user.clone(),
        group: args.group.clone(),
        daemon: args.daemon,
        threads: works,
        ..Default::default()
    };

    let mut server = Server::new_with_opt_and_conf(args, main_conf);
    let main_conf = server.configuration.clone();

    let proxy = AigwProxy::new(config.clone(), storage.clone(), geo_lite.clone());

    let mut proxy_service_http = http_proxy_service_with_name(&main_conf, proxy, "Aigw-http");
    if let Some(proxy) = proxy_service_http.app_logic_mut() {
        if let Some(opt) = &mut proxy.server_options {
            opt.h2c = true;
        } else {
            let mut opt = HttpServerOptions::default();
            opt.h2c = true;
            proxy.server_options = Some(opt);
        }
    }
    let addr = format!(":::{}", config.basic().http());
    proxy_service_http.add_tcp(addr.as_str());

    let proxy = AigwProxy::new(config.clone(), storage.clone(), geo_lite);
    let dynamic_cert = DynamicTlsAccept::new(storage);
    let mut tls_settings = TlsSettings::with_callbacks(Box::new(dynamic_cert))?;
    tls_settings.set_max_proto_version(Some(SslVersion::TLS1_3))?;
    tls_settings.set_min_proto_version(Some(SslVersion::TLS1_2))?;
    // TLS 1.2 requires setting cipher suites
    let _= tls_settings.set_cipher_list("TLS-CHACHA20-POLY1305-SHA256:TLS-AES-256-GCM-SHA384:TLS-AES-128-GCM-SHA256:HIGH:!aNULL:!MD5");
    tls_settings.enable_h2();
    let mut proxy_service_https = http_proxy_service_with_name(&main_conf, proxy, "Aigw-https");
    let addr = format!(":::{}", config.basic().https());
    proxy_service_https.add_tls_with_settings(addr.as_str(), None, tls_settings);

    server.bootstrap();
    server.add_service(proxy_service_http);
    server.add_service(proxy_service_https);
    server.add_service(aigwc_service);

    server.run(run_args(shutdown_tx));
    //

    info!("All runtimes exited, exiting now");
    std::process::exit(0)
}
