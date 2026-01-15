use jot_core::StorageEngine;

use crate::app::{missing_config_message, missing_jot_message, resolve_config_path, AppContext};
use crate::cli::DoctorArgs;
use crate::config::read_config;
use crate::ui::{badge, header, hint, kv, Badge, OutputMode, StepList};

pub fn handle_doctor(ctx: &AppContext, args: &DoctorArgs) -> anyhow::Result<()> {
    let ui_ctx = ctx.ui_context(false, None);

    let config_path = resolve_config_path()?;
    if !config_path.exists() {
        eprintln!("{}", missing_config_message(&config_path));
        return Err(anyhow::anyhow!("Jot is not initialized"));
    }

    let config = read_config(&config_path).map_err(|e| anyhow::anyhow!("Config error: {}", e))?;
    let jot_path = std::path::PathBuf::from(&config.jot.path);
    if !jot_path.exists() {
        eprintln!("{}", missing_jot_message(&jot_path));
        return Err(anyhow::anyhow!("Jot file missing"));
    }

    let (storage, _passphrase) = ctx.open_storage(args.no_input).map_err(|e| {
        anyhow::anyhow!(
            "Failed to open jot for diagnostics: {}\nHint: Set JOT_PASSPHRASE or run in a TTY.",
            e
        )
    })?;

    // Run integrity check first
    let integrity_result = storage.check_integrity();

    // Handle errors (always output, regardless of quiet)
    if let Err(ref err) = integrity_result {
        match ui_ctx.mode {
            OutputMode::Pretty => {
                println!("{}", header(&ui_ctx, "doctor", None));
                println!();

                let mut steps =
                    StepList::new(&ui_ctx, &["Config file", "Jot file", "Integrity check"]);
                steps.ok();
                steps.ok();
                steps.err();

                println!();
                println!("{}", badge(&ui_ctx, Badge::Err, "Doctor failed"));
                println!("  {}", kv(&ui_ctx, "Error", &err.to_string()));
                println!();
                println!(
                    "{}",
                    hint(
                        &ui_ctx,
                        "Restore from a backup or export data before retrying."
                    )
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("check=config ok");
                println!("check=jot ok");
                println!("check=integrity err");
                println!("error={}", err);
                println!("status=failed");
            }
        }
        return Err(anyhow::anyhow!("Doctor failed"));
    }

    // Handle success (respect quiet flag)
    if !ctx.quiet() {
        match ui_ctx.mode {
            OutputMode::Pretty => {
                println!("{}", header(&ui_ctx, "doctor", None));
                println!();

                let mut steps =
                    StepList::new(&ui_ctx, &["Config file", "Jot file", "Integrity check"]);
                steps.ok();
                steps.ok();
                steps.ok();

                println!();
                println!("{}", badge(&ui_ctx, Badge::Ok, "Jot is healthy"));
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("check=config ok");
                println!("check=jot ok");
                println!("check=integrity ok");
                println!("status=ok");
            }
        }
    }

    Ok(())
}
