use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct AigwConsoleArgs {
    /// Name of the person to greet
    #[arg(short, long)]
    pub config: Option<String>,

    #[arg(short, long)]
    pub log_file: Option<String>,

    #[arg(short, long)]
    pub install: bool,

    /// Run the process in the background
    #[structopt(long)]
    pub daemon: bool,

    /// Run as other user
    #[structopt(long)]
    pub user: Option<String>,

    /// Run as other group
    #[structopt(long)]
    pub group: Option<String>,

    #[structopt(long)]
    pub pid_file: Option<String>,

    #[structopt(long)]
    pub ui: Option<String>,
}

impl AigwConsoleArgs {
    pub fn do_parse() -> Self {
        AigwConsoleArgs::parse()
    }
}
