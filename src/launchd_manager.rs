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

        let _= file.write(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
    <dict>
        <key>Label</key>
        <string>{}</string>
        <key>ProgramArguments</key>
        <array>
            <string>{}</string>
        </array>
        <key>KeepAlive</key>
        <true/>
        <key>RunAtLoad</key>
        <true/>
        <key>StandardOutPath</key>
        <string>/tmp/app-activate.out.log</string>
        <key>StandardErrorPath</key>
        <string>/tmp/app-activate.err.log</string>
    </dict>
</plist>
"#,
                                  self.name, self.bin.to_string_lossy()).as_bytes());

        // Just record the result of each command and continue on error, because I'm not 100% sure
        // what is the best way to register a service due to sparse documentation...
        match run_cmd!(launchctl bootstrap gui/$id $plist) {
            Ok(_) => info!("Bootstrapped gui/{id}/{name}"),
            Err(why) => warn!("Failed to bootstrap gui/{id}/{name}: {why}"),
        }
        match run_cmd!(launchctl load -w $plist) {
            Ok(_) => info!("Registered app-activate"),
            Err(why) => warn!("Failed to load {name}: {why}"),
        }
        match run_cmd!(launchctl enable gui/$id/$name) {
            Ok(_) => info!("Enabled gui/{id}/{name}"),
            Err(why) => warn!("Failed to enable gui/{id}/{name}: {why}"),
        };
        match run_cmd!(launchctl start $name) {
            Ok(_) => info!("Started {name}"),
            Err(why) => warn!("Failed to start gui/{id}/{name}: {why}"),
        };
        Ok(())
    }

    pub fn unregister(&self) -> Result<()> {
        let name = self.name.clone();
        let plist = self.plist.clone();

        // Just record the result of each command and continue on error, because I'm not 100% sure
        // what is the best way to register a service due to sparse documentation...
        match run_cmd!(launchctl stop $name) {
            Ok(_) => info!("Stopped {name}"),
            Err(why) => warn!("Failed to stop {name}: {why}"),
        }
        match run_cmd!(launchctl unload -w $plist) {
            Ok(_) => info!("Unloaded {name}"),
            Err(why) => warn!("Failed to unload {name}: {why}"),
        };
        match remove_file(&plist) {
            Ok(_) => info!("Removed {plist:?}"),
            Err(why) => warn!("Failed to remove {plist:?}: {why}"),
        }
        Ok(())
    }
}
