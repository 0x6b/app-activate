use anyhow::Result;
use app_activate::{get_config, UsageReporter};
use clap::Parser;
use env_logger::Env;

use crate::args::Args;

mod args;

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let config = get_config(args.config)?;
    UsageReporter::new(&config)?.report()
}
