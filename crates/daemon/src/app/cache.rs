pub(super) use veila_session::services::cache::harden_existing_cache_root;

pub(super) fn spawn_background_cache_pruner() {
    veila_renderer::cache::spawn_cache_maintenance(std::time::Duration::from_secs(60));
}
