use ledger_core::storage::{NewComposition, StorageEngine};

use crate::app::AppContext;
use crate::cli::CompositionCreateArgs;

pub fn handle_create(ctx: &AppContext, args: &CompositionCreateArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;
    let metadata = storage.metadata()?;

    let mut new_composition = NewComposition::new(&args.name, metadata.device_id);
    if let Some(ref desc) = args.description {
        new_composition = new_composition.with_description(desc);
    }

    let composition_id = storage.create_composition(&new_composition)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        println!("Created composition '{}' ({})", args.name, composition_id);
    }
    Ok(())
}
