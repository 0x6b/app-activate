use std::{collections::HashMap, fs::read_to_string, path::Path};

use serde::Deserialize;
use toml::from_str;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub launcher: LauncherConfig,
}

#[derive(Debug, Deserialize)]
pub struct LauncherConfig {
    pub leader: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub primary: HashMap<String, String>,
}

fn default_timeout() -> u64 {
    600
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = read_to_string(path)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        let config: Self =
            from_str(&text).map_err(|error| format!("Invalid {}: {error}", path.display()))?;
        if !(100..=5_000).contains(&config.launcher.timeout_ms) {
            return Err("launcher.timeout_ms must be between 100 and 5000".into());
        }
        Ok(config)
    }
}

pub fn virtual_key(name: &str) -> Result<u32, String> {
    let upper = name.trim().to_ascii_uppercase();
    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Ok(u32::from(byte));
        }
    }
    if let Some(hex) = upper.strip_prefix("VK_") {
        return u32::from_str_radix(hex, 16).map_err(|_| format!("invalid virtual key: {name}"));
    }
    let key = match upper.as_str() {
        "CAPSLOCK" => 0x14,
        "SCROLLLOCK" => 0x91,
        "PAUSE" => 0x13,
        "INSERT" => 0x2D,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        _ if upper.starts_with('F') => upper[1..]
            .parse::<u32>()
            .ok()
            .filter(|number| (1..=24).contains(number))
            .map(|number| 0x6F + number)
            .ok_or_else(|| format!("unsupported key name: {name}"))?,
        _ => return Err(format!("unsupported key name: {name}")),
    };
    Ok(key)
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    use std::env::temp_dir;
    #[cfg(test)]
    use std::fs::remove_file;
    use std::{fs::write, process::id};

    use toml::from_str;

    use super::*;

    #[test]
    fn parses_named_and_character_keys() {
        assert_eq!(virtual_key("CapsLock").unwrap(), 0x14);
        assert_eq!(virtual_key("g").unwrap(), u32::from(b'G'));
        assert_eq!(virtual_key("F12").unwrap(), 0x7B);
        assert_eq!(virtual_key("VK_BA").unwrap(), 0xBA);
    }

    #[test]
    fn parses_launcher_table() {
        let config: Config = from_str(
            "[launcher]\nleader = \"CapsLock\"\n[launcher.primary]\ng = \"https://github.com\"",
        )
        .unwrap();
        assert_eq!(config.launcher.leader, "CapsLock");
        assert_eq!(config.launcher.timeout_ms, 600);
        assert_eq!(config.launcher.primary["g"], "https://github.com");
    }

    #[test]
    fn rejects_timeout_outside_supported_range() {
        let path = temp_dir().join(format!("app-activate-config-{}.toml", id()));
        write(
            &path,
            r#"
                [launcher]
                leader = "CapsLock"
                timeout_ms = 99
            "#,
        )
        .unwrap();

        let result = Config::load(&path);
        remove_file(path).unwrap();

        assert_eq!(result.unwrap_err(), "launcher.timeout_ms must be between 100 and 5000");
    }
}
