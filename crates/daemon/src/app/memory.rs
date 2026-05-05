#[cfg(target_os = "linux")]
pub(super) fn current_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(parse_vm_rss_kib)
}

#[cfg(not(target_os = "linux"))]
pub(super) fn current_rss_kib() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn parse_vm_rss_kib(line: &str) -> Option<u64> {
    let value = line.strip_prefix("VmRSS:")?.trim();
    let number = value.split_whitespace().next()?;
    number.parse().ok()
}
