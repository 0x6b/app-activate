use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum Target<'a> {
    PackagedApp(&'a str),
    Executable(&'a Path),
    Shell(&'a str),
}

pub fn classify(target: &str) -> Target<'_> {
    const PREFIX: &str = "shell:AppsFolder\\";
    if let Some((prefix, app_id)) = target.split_at_checked(PREFIX.len())
        && prefix.eq_ignore_ascii_case(PREFIX)
        && !app_id.is_empty()
    {
        return Target::PackagedApp(app_id);
    }

    let path = Path::new(target);
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        Target::Executable(path)
    } else {
        Target::Shell(target)
    }
}

pub fn executable_matches(actual: &Path, configured: &Path) -> bool {
    normalized(actual).eq_ignore_ascii_case(&normalized(configured))
        || file_name(actual).eq_ignore_ascii_case(file_name(configured))
}

fn normalized(path: &Path) -> String {
    let rendered = path.to_string_lossy();
    rendered.strip_prefix(r"\\?\").unwrap_or(&rendered).to_owned()
}

fn file_name(path: &Path) -> &str {
    path.to_str()
        .unwrap_or_default()
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_packaged_apps_case_insensitively() {
        assert_eq!(
            classify(r"SHELL:APPSFOLDER\Microsoft.WindowsTerminal_8wekyb3d8bbwe!App"),
            Target::PackagedApp("Microsoft.WindowsTerminal_8wekyb3d8bbwe!App")
        );
    }

    #[test]
    fn classifies_executables_case_insensitively() {
        assert_eq!(
            classify(r"C:\Tools\app.EXE"),
            Target::Executable(Path::new(r"C:\Tools\app.EXE"))
        );
    }

    #[test]
    fn classifies_shell_and_url_targets() {
        assert_eq!(classify("https://example.com"), Target::Shell("https://example.com"));
        assert_eq!(classify("shell:Downloads"), Target::Shell("shell:Downloads"));
        assert_eq!(classify(r"shell:AppsFolder\"), Target::Shell(r"shell:AppsFolder\"));
    }

    #[test]
    fn matches_paths_case_insensitively() {
        assert!(executable_matches(
            Path::new(r"C:\Program Files\Alacritty\alacritty.exe"),
            Path::new(r"c:\program files\alacritty\Alacritty.exe")
        ));
    }

    #[test]
    fn matches_executable_names_across_shim_paths() {
        assert!(executable_matches(
            Path::new(r"C:\Program Files\Alacritty\alacritty.exe"),
            Path::new(r"C:\Users\user\scoop\shims\alacritty.exe")
        ));
    }

    #[test]
    fn rejects_different_executable_names() {
        assert!(!executable_matches(
            Path::new(r"C:\Program Files\Alacritty\alacritty.exe"),
            Path::new(r"C:\Windows\notepad.exe")
        ));
    }
}
