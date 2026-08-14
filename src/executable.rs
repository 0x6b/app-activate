use std::path::Path;

pub fn matches(actual: &Path, configured: &Path) -> bool {
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
    fn matches_paths_case_insensitively() {
        assert!(matches(
            Path::new(r"C:\Program Files\Alacritty\alacritty.exe"),
            Path::new(r"c:\program files\alacritty\Alacritty.exe")
        ));
    }

    #[test]
    fn matches_executable_names_across_shim_paths() {
        assert!(matches(
            Path::new(r"C:\Program Files\Alacritty\alacritty.exe"),
            Path::new(r"C:\Users\user\scoop\shims\alacritty.exe")
        ));
    }

    #[test]
    fn rejects_different_executable_names() {
        assert!(!matches(
            Path::new(r"C:\Program Files\Alacritty\alacritty.exe"),
            Path::new(r"C:\Windows\notepad.exe")
        ));
    }
}
