use crate::cache::run_cache_daemon;

pub fn handle_internal_cache_daemon(ttl: u64, socket: &str) -> anyhow::Result<()> {
    let socket_path = std::path::PathBuf::from(socket);
    run_cache_daemon(std::time::Duration::from_secs(ttl), &socket_path)?;
    Ok(())
}
