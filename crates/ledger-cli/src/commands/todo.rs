use uuid::Uuid;

use ledger_core::storage::{Entry, NewEntry, StorageEngine};

use crate::app::{exit_not_found_with_hint, AppContext};
use crate::cli::{TodoArgs, TodoListArgs, TodoSubcommand, TodoUpdateArgs};
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, blank_line, header, hint, kv, print, short_id, Badge, OutputMode};

pub fn handle_todo(ctx: &AppContext, args: &TodoArgs) -> anyhow::Result<()> {
    match &args.command {
        TodoSubcommand::List(list_args) => handle_list(ctx, list_args),
        TodoSubcommand::Done(update_args) => handle_update(ctx, update_args, true),
        TodoSubcommand::Undo(update_args) => handle_update(ctx, update_args, false),
    }
}

fn handle_list(ctx: &AppContext, args: &TodoListArgs) -> anyhow::Result<()> {
    let (storage, passphrase) = ctx.open_storage(args.no_input)?;
    let entry_id =
        Uuid::parse_str(&args.id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
    let entry = storage.get_entry(&entry_id)?.unwrap_or_else(|| {
        exit_not_found_with_hint(
            "Entry not found",
            "Hint: Run `ledger list --last 7d` to find entry IDs.",
        )
    });

    let tasks = extract_tasks(&entry)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        let ui_ctx = ctx.ui_context(false, None);
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(&ui_ctx, &header(&ui_ctx, "todo", None));
                blank_line(&ui_ctx);
                print(&ui_ctx, &kv(&ui_ctx, "Entry", &short_id(&entry.id)));
                blank_line(&ui_ctx);
                for (idx, task) in tasks.iter().enumerate() {
                    let marker = if task.done {
                        if ui_ctx.unicode {
                            "[\u{2713}]"
                        } else {
                            "[x]"
                        }
                    } else {
                        "[ ]"
                    };
                    let marker = if task.done {
                        styled(marker, styles::success(), ui_ctx.color)
                    } else {
                        marker.to_string()
                    };
                    println!("{} {}. {}", marker, idx + 1, task.text);
                }
            }
            OutputMode::Plain | OutputMode::Json => {
                for (idx, task) in tasks.iter().enumerate() {
                    println!(
                        "task={} done={} text={}",
                        idx + 1,
                        task.done,
                        task.text.replace('\n', " ")
                    );
                }
            }
        }
    }

    Ok(())
}

fn handle_update(ctx: &AppContext, args: &TodoUpdateArgs, done: bool) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(args.no_input)?;
    let entry_id =
        Uuid::parse_str(&args.id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
    let entry = storage.get_entry(&entry_id)?.unwrap_or_else(|| {
        exit_not_found_with_hint(
            "Entry not found",
            "Hint: Run `ledger list --last 7d` to find entry IDs.",
        )
    });

    let mut data = entry.data.clone();
    let items = data
        .get_mut("items")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("Todo entry missing 'items' task list"))?;

    if args.index == 0 || args.index > items.len() {
        return Err(anyhow::anyhow!(
            "Invalid task index {} (expected 1-{})",
            args.index,
            items.len()
        ));
    }
    let idx = args.index - 1;
    let item = items
        .get_mut(idx)
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow::anyhow!("Invalid task format at index {}", args.index))?;
    let text = item
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    item.insert("done".to_string(), serde_json::Value::Bool(done));

    let metadata = storage.metadata()?;
    let new_entry = NewEntry::new(
        entry.entry_type_id,
        entry.schema_version,
        data,
        metadata.device_id,
    )
    .with_tags(entry.tags.clone())
    .with_supersedes(entry.id);

    let new_entry_id = storage.insert_entry(&new_entry)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        let ui_ctx = ctx.ui_context(false, None);
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(
                    &ui_ctx,
                    &badge(
                        &ui_ctx,
                        Badge::Ok,
                        if done {
                            "Task completed"
                        } else {
                            "Task reopened"
                        },
                    ),
                );
                print(&ui_ctx, &kv(&ui_ctx, "Entry", &short_id(&new_entry_id)));
                if !text.is_empty() {
                    print(&ui_ctx, &kv(&ui_ctx, "Task", &text));
                }
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &hint(
                        &ui_ctx,
                        &format!(
                            "ledger todo list {}  \u{00B7}  ledger show {}",
                            short_id(&new_entry_id),
                            short_id(&new_entry_id)
                        ),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("entry_id={}", new_entry_id);
                println!("task_index={}", args.index);
                println!("done={}", done);
            }
        }
    }

    Ok(())
}

struct TodoTask {
    text: String,
    done: bool,
}

fn extract_tasks(entry: &Entry) -> anyhow::Result<Vec<TodoTask>> {
    let items = entry
        .data
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Todo entry missing 'items' task list"))?;

    let mut tasks = Vec::new();
    for item in items {
        let obj = item
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid task format in items"))?;
        let text = obj
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let done = obj.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
        tasks.push(TodoTask { text, done });
    }
    Ok(tasks)
}
