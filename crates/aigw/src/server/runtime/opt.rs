use clap::Parser;
use pingora_core::prelude::Opt;

#[derive(Parser, Debug, Default)]
#[clap(name="basic", long_about=None)]
pub struct ServerOpt {
    #[clap(short, long, help="Upgrade from a running server.", long_help=None)]
    pub upgrade: bool,

    #[clap(short, long)]
    pub daemon: bool,

    #[clap(long, hide = true)]
    pub nocapture: bool,
    #[clap(
        short,
        long,
        help = "This flag is useful for upgrading service where the user wants \
                to make sure the new service can start before shutting down \
                the old server process.",
        long_help = None
    )]
    pub test: bool,

    #[clap(short, long, help="The path to the server configuration file.", long_help=None)]
    pub config: Option<String>,

    #[clap(long, help="MaxMind's GeoLite2 Country databases.", long_help=None)]
    pub geo_lite: Option<String>,

    #[arg(short, long)]
    pub log_dir: Option<String>,

    #[arg(short, long)]
    pub ebpf: Option<String>,

    /// Run as other user
    #[structopt(long)]
    pub user: Option<String>,

    /// Run as other group
    #[structopt(long)]
    pub group: Option<String>,

    #[structopt(long)]
    pub pid_file: Option<String>,
}

impl From<ServerOpt> for Option<Opt> {
    fn from(val: ServerOpt) -> Self {
        Some(Opt {
            upgrade: val.upgrade,
            daemon: val.daemon,
            nocapture: val.nocapture,
            test: val.test,
            conf: None,
        })
    }
}

impl ServerOpt {
    pub fn parse_args() -> Self {
        ServerOpt::parse()
    }
}
