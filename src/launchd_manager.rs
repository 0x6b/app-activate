use std::{
    fs::{File, remove_file},
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
    pub fn new(name: &str) -> Result<Self> {
        let id = run_fun!(/usr/bin/id -u)?;
        let home_dir = home_dir().unwrap();
        let bin = home_dir.join(".cargo/bin").join(name);
        let plist = home_dir.join("Library/LaunchAgents").join(format!("{name}.plist"));
        Ok(Self { name: name.to_string(), id, bin, plist })
    }

    pub fn register(&self) -> Result<()> {
        let (name, id, plist) = (&self.name, &self.id, &self.plist);
        let mut file = File::options().write(true).create(true).truncate(true).open(plist)?;

        file.write_all(
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
    <dict>
        <key>Label</key>
        <string>{}</string>
        <key>ProcessType</key>
        <string>Interactive</string>
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
                self.name,
                self.bin.to_string_lossy()
            )
            .as_bytes(),
        )?;

        self.run_launchctl("bootstrap", format!("gui/{id} {}", plist.display()));
        self.run_launchctl("load", format!("-w {}", plist.display()));
        self.run_launchctl("enable", format!("gui/{id}/{name}"));
        self.run_launchctl("start", name.to_string());
        Ok(())
    }

    pub fn unregister(&self) -> Result<()> {
        let (name, plist) = (&self.name, &self.plist);

        self.run_launchctl("stop", name.to_string());
        self.run_launchctl("unload", format!("-w {}", plist.display()));

        match remove_file(plist) {
            Ok(()) => info!("Removed {plist:?}"),
            Err(why) => warn!("Failed to remove {plist:?}: {why}"),
        }
        Ok(())
    }

    fn run_launchctl(&self, cmd: &str, args: String) {
        match run_cmd!(launchctl $cmd $args) {
            Ok(()) => info!("launchctl {cmd} {args}"),
            Err(why) => warn!("Failed: launchctl {cmd} {args}: {why}"),
        }
    }
}
