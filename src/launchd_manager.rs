use std::{
    fs::{remove_file, File},
    io::Write,
    path::PathBuf,
};

use anyhow::Result;
use cmd_lib::{run_cmd, run_fun};
use dirs::home_dir;
use log::{info, warn};

#[derive(Debug)]
pub struct LaunchdManager {
    name: String,
    id: String,
    bin: PathBuf,
    plist: PathBuf,
}

impl LaunchdManager {
    pub fn new(name: &str) -> Self {
        let id = run_fun!(/usr/bin/id -u).unwrap();
        let home_dir = home_dir().unwrap();
        let bin = home_dir.join(".cargo").join("bin").join(name);
        let plist = home_dir
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{name}.plist"));
        Self { name: name.to_string(), id, bin, plist }
    }

    pub fn register(&self) -> Result<()> {
        let name = self.name.clone();
        let id = self.id.clone();
        let plist = self.plist.clone();
        let mut file = File::options().write(true).create(true).truncate(true).open(&plist)?;

        let _= file.write(format!(r#"<?xml version="1.0"
encoding="UTF-8"?> <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>StandardOutPath</key>
    <string>/tmp/app-activate.out.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/app-activate.err.log</string>
</dict>
</plist>
"#,
                                  self.name, self.bin.to_string_lossy()).as_bytes());
        if run_cmd!(launchctl bootstrap gui/$id $plist).is_err() {
            warn!("Failed to bootstrap {name}")
        }
        match run_cmd!(launchctl load -w $plist) {
            Ok(_) => info!("Registered app-activate"),
            Err(why) => warn!("Failed to load {name}: {why}"),
        }
        match run_cmd!(launchctl start $name) {
            Ok(_) => info!("Started {name}"),
            Err(why) => warn!("Failed to start {name}: {why}"),
        };
        Ok(())
    }

    pub fn unregister(&self) -> Result<()> {
        let name = self.name.clone();
        let plist = self.plist.clone();
        if run_cmd!(launchctl stop $name).is_err() {
            warn!("Failed to stop {name}")
        }
        if run_cmd!(launchctl unload -w $plist).is_err() {
            warn!("Failed to unload {name}")
        };
        if remove_file(&plist).is_ok() {
            info!("Removed {}", plist.to_string_lossy())
        } else {
            warn!("Failed to remove {}", plist.to_string_lossy())
        }
        Ok(())
    }
}
