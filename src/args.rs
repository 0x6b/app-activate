use std::path::PathBuf;

use clap::Parser;
use env_logger::Env;
use log::debug;

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct Args {
    /// Path to the configuration file. Defaults to `$XDG_CONFIG_HOME/app-activate/config.toml`.
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Subcommand to run.
    #[clap(subcommand)]
    pub command: Option<Command>,
}

impl Args {
    pub fn new() -> Self {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
        let args = Self::parse();
        debug!("{args:?}");
        args
    }
}

#[derive(Debug, Clone, Parser)]
pub enum Command {
    /// Start the application. Default if no subcommand is provided.
    Start,

    /// Register the application to start on login.
    Register,

    /// Unregister the application from starting on login.
    Unregister,
}
