use anyhow::Result;
use app_activate::{UsageReporter, get_config};

use crate::args::Args;

mod args;

fn main() -> Result<()> {
    UsageReporter::new(&get_config(Args::new().config)?)?.report()
}
