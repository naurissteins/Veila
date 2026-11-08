use std::path::{Path, PathBuf};

pub const LEGACY_IDLE_SERVICE: &str = "veila-idle.service";

/// Enablement symlinks named `unit` in the user's `*.wants/` directories, including dangling ones.
pub fn enablement_links(unit_config_dir: &Path, unit: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(unit_config_dir) else {
        return Vec::new();
    };

    let mut links: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "wants")
        })
        .map(|path| path.join(unit))
        .filter(|link| link.symlink_metadata().is_ok())
        .collect();
    links.sort();
    links
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("systemd/user")
}

#[cfg(test)]
mod tests {
    use super::enablement_links;

    #[test]
    fn finds_enablement_links_for_a_unit() {
        let root = std::env::temp_dir().join(format!(
            "veila-doctor-links-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let wants = root.join("graphical-session.target.wants");
        std::fs::create_dir_all(&wants).expect("wants dir");
        std::fs::create_dir_all(root.join("default.target.wants")).expect("other wants dir");
        std::os::unix::fs::symlink(
            "/usr/lib/systemd/user/veilad.service",
            wants.join("veilad.service"),
        )
        .expect("dangling legacy link");
        std::os::unix::fs::symlink(
            "/usr/lib/systemd/user/veila.service",
            wants.join("veila.service"),
        )
        .expect("current link");

        assert_eq!(
            enablement_links(&root, "veilad.service"),
            vec![wants.join("veilad.service")]
        );
        assert!(enablement_links(&root, "veila-idle.service").is_empty());
        assert!(enablement_links(&root.join("missing"), "veilad.service").is_empty());

        std::fs::remove_dir_all(root).ok();
    }
}
