use ledger_core::storage::{CompositionFilter, StorageEngine};

use crate::app::AppContext;
use crate::cli::CompositionListArgs;
use crate::ui::{blank_line, header, hint, print, short_id, simple_table, Column, OutputMode};

pub fn handle_list(ctx: &AppContext, args: &CompositionListArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let mut filter = CompositionFilter::new();
    if let Some(limit) = args.limit {
        filter = filter.limit(limit);
    }

    let compositions = storage.list_compositions(&filter)?;

    // Create UI context
    let ui_ctx = ctx.ui_context(args.json, None);

    // Handle JSON output
    if ui_ctx.mode.is_json() {
        let json_output: Vec<_> = compositions
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id.to_string(),
                    "name": c.name,
                    "description": c.description,
                    "created_at": c.created_at.to_rfc3339(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_output)?);
        return Ok(());
    }

    // Empty result handling
    if compositions.is_empty() {
        if !ctx.quiet() {
            match ui_ctx.mode {
                OutputMode::Pretty => {
                    print(&ui_ctx, &header(&ui_ctx, "compositions", None));
                    blank_line(&ui_ctx);
                    print(&ui_ctx, &hint(&ui_ctx, "No compositions found."));
                }
                OutputMode::Plain | OutputMode::Json => {
                    println!("count=0");
                }
            }
        }
        return Ok(());
    }

    // Render compositions
    match ui_ctx.mode {
        OutputMode::Pretty => {
            print(&ui_ctx, &header(&ui_ctx, "compositions", None));
            blank_line(&ui_ctx);

            let columns = [
                Column::new("Name"),
                Column::new("Description"),
                Column::new("ID"),
            ];

            let rows: Vec<Vec<String>> = compositions
                .iter()
                .map(|c| {
                    vec![
                        c.name.clone(),
                        c.description.clone().unwrap_or_default(),
                        short_id(&c.id),
                    ]
                })
                .collect();

            print(&ui_ctx, &simple_table(&ui_ctx, &columns, &rows));
            blank_line(&ui_ctx);
            print(
                &ui_ctx,
                &hint(&ui_ctx, &format!("{} compositions", compositions.len())),
            );
        }
        OutputMode::Plain | OutputMode::Json => {
            for comp in &compositions {
                let desc = comp.description.as_deref().unwrap_or("");
                println!("{} {} {}", comp.id, comp.name, desc);
            }
        }
    }

    Ok(())
}
