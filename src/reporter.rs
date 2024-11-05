use app_activate::{get_config, UsageReporter};

use crate::args::Args;

mod args;

fn main() -> anyhow::Result<()> {
    UsageReporter::new(&get_config(Args::new().config)?)?.report()
}
