use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PamService {
    pub name: &'static str,
    pub fallback: bool,
}

pub fn selected_service() -> Option<PamService> {
    select_service_at(Path::new("/etc/pam.d"))
}

fn select_service_at(root: &Path) -> Option<PamService> {
    for name in ["veila", "system-auth", "common-auth"] {
        if root.join(name).is_file() {
            return Some(PamService {
                name,
                fallback: name != "veila",
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::{PamService, select_service_at};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn with_pam_dir(test: impl FnOnce(&std::path::Path)) {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("veila-pam-service-{}-{id}", std::process::id()));
        fs::create_dir(&root).expect("temporary PAM directory");
        test(&root);
        fs::remove_dir_all(root).expect("remove temporary PAM directory");
    }

    #[test]
    fn prefers_veila_service_when_installed() {
        with_pam_dir(|root| {
            fs::write(root.join("veila"), "auth required pam_unix.so\n").expect("veila service");
            fs::write(root.join("system-auth"), "auth required pam_unix.so\n")
                .expect("system service");
            assert_eq!(
                select_service_at(root),
                Some(PamService {
                    name: "veila",
                    fallback: false
                })
            );
        });
    }

    #[test]
    fn uses_system_auth_when_veila_service_is_missing() {
        with_pam_dir(|root| {
            fs::write(root.join("system-auth"), "auth required pam_unix.so\n")
                .expect("system service");
            assert_eq!(
                select_service_at(root),
                Some(PamService {
                    name: "system-auth",
                    fallback: true
                })
            );
        });
    }

    #[test]
    fn uses_common_auth_when_only_debian_stack_is_present() {
        with_pam_dir(|root| {
            fs::write(root.join("common-auth"), "auth required pam_unix.so\n")
                .expect("common service");
            assert_eq!(
                select_service_at(root),
                Some(PamService {
                    name: "common-auth",
                    fallback: true
                })
            );
        });
    }

    #[test]
    fn reports_missing_service_when_no_supported_stack_exists() {
        with_pam_dir(|root| assert_eq!(select_service_at(root), None));
    }
}
