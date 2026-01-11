use crate::cache::run_cache_daemon;
use crate::cli::InternalCacheDaemonArgs;

pub fn handle_internal_cache_daemon(args: &InternalCacheDaemonArgs) -> anyhow::Result<()> {
    let socket_path = std::path::PathBuf::from(&args.socket);
    run_cache_daemon(std::time::Duration::from_secs(args.ttl), &socket_path)?;
    Ok(())
}
