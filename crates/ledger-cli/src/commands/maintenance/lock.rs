use crate::app::AppContext;
use crate::cache::{cache_clear, cache_socket_path};

pub fn handle_lock(ctx: &AppContext) -> anyhow::Result<()> {
    if let Ok(socket_path) = cache_socket_path() {
        let _ = cache_clear(&socket_path);
    }
    if !ctx.quiet() {
        println!("Passphrase cache cleared.");
    }
    Ok(())
}
