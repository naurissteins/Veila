use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub(super) fn expand_home_path(path: &Path) -> PathBuf {
    expand_home_path_with_home(path, std::env::var_os("HOME").as_deref())
}

fn expand_home_path_with_home(path: &Path, home: Option<&OsStr>) -> PathBuf {
    // Strip a whole component so repeated separators cannot replace HOME when joined.
    if let Ok(rest) = path.strip_prefix("~")
        && let Some(home) = home
    {
        if rest.as_os_str().is_empty() {
            return PathBuf::from(home);
        }
        return PathBuf::from(home).join(rest);
    }

    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_only_a_leading_tilde_component() {
        for (input, expected) in [
            ("~", "/home/test"),
            ("~/", "/home/test"),
            ("~/Pictures/a.png", "/home/test/Pictures/a.png"),
            ("~//fragment.toml", "/home/test/fragment.toml"),
            ("~/./fragment.toml", "/home/test/fragment.toml"),
            ("~/../fragment.toml", "/home/test/../fragment.toml"),
            ("~/a/../fragment.toml", "/home/test/a/../fragment.toml"),
        ] {
            assert_eq!(
                expand_home_path_with_home(Path::new(input), Some(OsStr::new("/home/test"))),
                Path::new(expected),
                "{input}"
            );
        }
    }

    #[test]
    fn leaves_other_paths_byte_for_byte_unchanged() {
        for input in ["", "a//b", "/tmp/~/a", "~user/a", "~name", "./~/a"] {
            assert_eq!(
                expand_home_path_with_home(Path::new(input), Some(OsStr::new("/home/test")))
                    .as_os_str(),
                OsStr::new(input)
            );
        }
    }

    #[test]
    fn leaves_tilde_paths_byte_for_byte_unchanged_without_home() {
        for input in ["~", "~/", "~//fragment.toml", "~/./a", "~/../a"] {
            assert_eq!(
                expand_home_path_with_home(Path::new(input), None).as_os_str(),
                OsStr::new(input)
            );
        }
    }

    #[test]
    fn keeps_empty_and_relative_home_values() {
        for (home, expected) in [
            ("", "fragment.toml"),
            ("relative", "relative/fragment.toml"),
        ] {
            assert_eq!(
                expand_home_path_with_home(Path::new("~/fragment.toml"), Some(OsStr::new(home))),
                Path::new(expected)
            );
        }
    }

    #[test]
    fn bare_tilde_preserves_home_bytes() {
        for home in ["", "relative", "/home/test", "/home/test/"] {
            assert_eq!(
                expand_home_path_with_home(Path::new("~"), Some(OsStr::new(home))).as_os_str(),
                OsStr::new(home)
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn preserves_non_utf8_home_and_path_bytes() {
        use std::os::unix::ffi::OsStrExt;

        assert_eq!(
            expand_home_path_with_home(
                Path::new(OsStr::from_bytes(b"~/image-\xff.png")),
                Some(OsStr::from_bytes(b"/home/\xfe")),
            )
            .as_os_str()
            .as_bytes(),
            b"/home/\xfe/image-\xff.png"
        );
    }
}
