use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub leader_key: String,
    pub applications: BTreeMap<String, PathBuf>,
    pub timeout_ms: u64,
}
