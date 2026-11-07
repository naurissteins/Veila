use std::{
    io::{self, Read},
    os::unix::{fs::OpenOptionsExt, net::UnixStream},
    path::Path,
    time::{Duration, Instant},
};
use veila_common::{
    AppConfig, BatterySnapshot,
    session::{SupervisorMessage, frame},
};

pub(crate) fn start(config: AppConfig) -> io::Result<UnixStream> {
    let (reader, mut writer) = UnixStream::pair()?;
    reader.set_nonblocking(true)?;
    std::thread::Builder::new()
        .name("veila-initial-data".into())
        .spawn(move || {
            let started = Instant::now();
            if config.battery.enabled {
                let snapshot = battery(Path::new("/sys/class/power_supply"), started);
                let _ = frame::write(&mut writer, &SupervisorMessage::UpdateBattery { snapshot });
            }
            if config.weather.enabled && started.elapsed() < Duration::from_millis(3) {
                let snapshot = veila_common::cache::resolve_root(None).and_then(|root| {
                    veila_common::weather::cache::load_snapshot_from(
                        &config,
                        &root.join("weather"),
                        None,
                    )
                    .ok()
                    .flatten()
                });
                let _ = frame::write(&mut writer, &SupervisorMessage::UpdateWeather { snapshot });
            }
        })?;
    Ok(reader)
}

fn battery(root: &Path, started: Instant) -> Option<BatterySnapshot> {
    for entry in std::fs::read_dir(root).ok()?.take(32) {
        if started.elapsed() > Duration::from_millis(3) {
            return None;
        }
        let path = entry.ok()?.path();
        if small(&path.join("type"))?.trim() != "Battery" {
            continue;
        }
        let percent = small(&path.join("capacity"))?
            .trim()
            .parse::<u8>()
            .ok()?
            .min(100);
        let status = small(&path.join("status"))?;
        return Some(BatterySnapshot {
            percent,
            charging: status.trim() == "Charging",
        });
    }
    None
}
fn small(path: &Path) -> Option<String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let mut text = String::new();
    file.take(65).read_to_string(&mut text).ok()?;
    (text.len() <= 64).then_some(text)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn battery_reads_are_bounded_and_do_not_require_dbus() {
        let root = std::env::temp_dir().join(format!("veila-snapshot-{}", std::process::id()));
        let battery_dir = root.join("BAT0");
        std::fs::create_dir_all(&battery_dir).expect("dir");
        for (name, text) in [
            ("type", "Battery\n"),
            ("capacity", "75\n"),
            ("status", "Charging\n"),
        ] {
            std::fs::write(battery_dir.join(name), text).expect("file");
        }
        assert_eq!(
            battery(&root, Instant::now()),
            Some(BatterySnapshot {
                percent: 75,
                charging: true
            })
        );
        std::fs::write(battery_dir.join("capacity"), "x".repeat(70)).expect("oversized");
        assert!(battery(&root, Instant::now()).is_none());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
