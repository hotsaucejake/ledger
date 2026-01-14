use crate::app::AppContext;
use crate::cache::{cache_clear, cache_socket_path};
use crate::ui::{badge, blank_line, header, kv, print, Badge, OutputMode};

pub fn handle_lock(ctx: &AppContext) -> anyhow::Result<()> {
    if let Ok(socket_path) = cache_socket_path() {
        let _ = cache_clear(&socket_path);
    }

    if !ctx.quiet() {
        let ui_ctx = ctx.ui_context(false, None);
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(&ui_ctx, &header(&ui_ctx, "lock", None));
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &badge(&ui_ctx, Badge::Ok, "Passphrase cache cleared"),
                );
                print(&ui_ctx, &kv(&ui_ctx, "Cache", "empty"));
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("cache=empty");
            }
        }
    }

    Ok(())
}
