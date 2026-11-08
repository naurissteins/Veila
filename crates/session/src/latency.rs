pub(crate) fn print_latency_report(report: &veila_common::ipc::LockLatencyReport, verbose: bool) {
    println!("latency_report=true");
    println!("config_load_ms={}", report.config_load_ms);
    println!(
        "control_channel_setup_ms={}",
        report.control_channel_setup_ms
    );
    println!("curtain_spawn_ms={}", report.curtain_spawn_ms);
    println!("curtain_ready_wait_ms={}", report.curtain_ready_wait_ms);
    println!("activation_total_ms={}", report.activation_total_ms);
    if verbose {
        println!("latency_report_mode=verbose");
        println!("config_load_us={}", report.config_load_us);
        println!(
            "control_channel_setup_us={}",
            report.control_channel_setup_us
        );
        println!("curtain_spawn_us={}", report.curtain_spawn_us);
        println!("curtain_ready_wait_us={}", report.curtain_ready_wait_us);
        println!("activation_total_us={}", report.activation_total_us);
    }

    let Some(curtain) = report.curtain.as_ref() else {
        println!("curtain_report=unavailable");
        return;
    };

    println!("curtain_report=ok");
    println!("curtain_wayland_connect_ms={}", curtain.wayland_connect_ms);
    if verbose {
        println!("curtain_wayland_connect_us={}", curtain.wayland_connect_us);
    }
    println!("curtain_registry_ms={}", curtain.registry_ms);
    if verbose {
        println!("curtain_registry_us={}", curtain.registry_us);
    }
    println!("curtain_event_loop_ms={}", curtain.event_loop_ms);
    if verbose {
        println!("curtain_event_loop_us={}", curtain.event_loop_us);
    }
    println!("curtain_app_init_ms={}", curtain.app_init_ms);
    if verbose {
        println!("curtain_app_init_us={}", curtain.app_init_us);
    }
    println!("curtain_lock_request_ms={}", curtain.lock_request_ms);
    if verbose {
        println!("curtain_lock_request_us={}", curtain.lock_request_us);
    }
    println!(
        "curtain_startup_prepared_ms={}",
        curtain.startup_prepared_ms
    );
    if verbose {
        println!(
            "curtain_startup_prepared_us={}",
            curtain.startup_prepared_us
        );
    }
    println!(
        "first_surface_configured_ms={}",
        optional_time(curtain.first_surface_configured_ms)
    );
    if verbose {
        println!(
            "first_surface_configured_us={}",
            optional_time(curtain.first_surface_configured_us)
        );
    }
    println!(
        "all_surfaces_configured_ms={}",
        optional_time(curtain.all_surfaces_configured_ms)
    );
    if verbose {
        println!(
            "all_surfaces_configured_us={}",
            optional_time(curtain.all_surfaces_configured_us)
        );
        println!(
            "placeholder_committed_us={}",
            optional_time(curtain.placeholder_committed_us)
        );
    }
    println!(
        "session_locked_ms={}",
        optional_time(curtain.session_locked_ms)
    );
    if verbose {
        println!(
            "session_locked_us={}",
            optional_time(curtain.session_locked_us)
        );
    }
    println!("first_frame_ms={}", optional_time(curtain.first_frame_ms));
    if verbose {
        println!("first_frame_us={}", optional_time(curtain.first_frame_us));
    }
    println!(
        "ready_notified_ms={}",
        optional_time(curtain.ready_notified_ms)
    );
    if verbose {
        println!(
            "ready_notified_us={}",
            optional_time(curtain.ready_notified_us)
        );
        print_verbose_latency_summary(report);
    }
    println!("surface_count={}", curtain.surface_count);
}

fn optional_time(value: Option<u64>) -> String {
    value.map_or_else(|| String::from("none"), |value| value.to_string())
}

fn print_verbose_latency_summary(report: &veila_common::ipc::LockLatencyReport) {
    let Some(curtain) = report.curtain.as_ref() else {
        return;
    };

    if let (Some(first_frame_us), Some(session_locked_us)) =
        (curtain.first_frame_us, curtain.session_locked_us)
    {
        println!(
            "first_frame_to_session_locked_us={}",
            session_locked_us.saturating_sub(first_frame_us)
        );
    }

    if let (Some(configured_us), Some(session_locked_us)) = (
        curtain.all_surfaces_configured_us,
        curtain.session_locked_us,
    ) {
        println!(
            "all_surfaces_to_session_locked_us={}",
            session_locked_us.saturating_sub(configured_us)
        );
    }
}
