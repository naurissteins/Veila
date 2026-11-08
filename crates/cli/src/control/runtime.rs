use anyhow::Result;
pub(super) fn print_status() -> Result<()> {
    let status = veila_session::runtime_status()?;
    println!("state={}", status.state);
    println!("active_lock={}", status.curtain.is_some());
    println!("curtain_running={}", status.curtain.is_some());
    println!("supervisor_running={}", status.supervisor.is_some());
    println!("idle_running={}", status.idle.is_some());
    println!("live_reload_available={}", status.supervisor.is_some());
    for (name, process) in [("supervisor", status.supervisor), ("idle", status.idle)] {
        if let Some(process) = process {
            println!("{name}_pid={}", process.pid);
            println!(
                "{name}_config={}",
                process
                    .config
                    .as_deref()
                    .map(|p| p.display().to_string())
                    .as_deref()
                    .unwrap_or("defaults")
            );
        }
    }
    Ok(())
}
pub(super) fn reload_config() -> Result<()> {
    let count = veila_session::reload()?;
    println!("reload_requested={count}");
    if count == 0 {
        println!("effect=changes will apply on next lock");
    } else {
        println!("effect=reload requested; invalid changes retain the last good config");
    }
    Ok(())
}
