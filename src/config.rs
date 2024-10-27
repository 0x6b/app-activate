use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use notify::{recommended_watcher, Event, Watcher};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub leader_key: String,
    pub applications: BTreeMap<String, PathBuf>,
    pub timeout_ms: u64,
}

pub fn watch_config<P>(path: P, tx: Sender<()>) -> notify::Result<notify::RecommendedWatcher>
where
    P: AsRef<Path>,
{
    let path = path.as_ref().to_path_buf();
    let mut watcher = recommended_watcher(move |result: Result<Event, _>| {
        if let Ok(event) = result {
            if event.kind.is_modify() {
                let _ = tx.send(());
            }
        }
    })?;

    let watch_path = path.parent().unwrap_or(&path);
    watcher.watch(watch_path, notify::RecursiveMode::NonRecursive)?;

    Ok(watcher)
}
