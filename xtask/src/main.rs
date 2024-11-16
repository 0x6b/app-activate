use std::process::exit;

use clap::Parser;
use cmd_lib::run_cmd;
use env_logger::Env;
use log::{error, info};

#[derive(Debug, Parser)]
struct Args {
    #[clap(subcommand)]
    command: SubCommand,
}

#[derive(Debug, Parser)]
enum SubCommand {
    /// Update app-activate with the latest version
    Update,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let Args { command } = Args::parse();

    let doit = |result, success, failure| match result {
        Ok(_) => info!("{success}"),
        Err(why) => {
            error!("{failure}: {why}");
            exit(1)
        }
    };

    match command {
        SubCommand::Update => {
            doit(
                run_cmd!(app-activate unregister),
                "Successfully unregistered current app-activate from launched",
                "Failed to unregister app-activate",
            );

            let root_dir = env!("CARGO_WORKSPACE_DIR");
            doit(
                run_cmd!(cargo install --path=$root_dir),
                "Successfully installed new app-activate binaries",
                "Failed to install app-activate",
            );
            doit(
                run_cmd!(app-activate register),
                "Successfully registered app-activate",
                "Failed to register app-activate",
            );
            info!("Successfully updated app-activate")
        }
    }
}
