mod command;
mod context;

use anyhow::Result;

use crate::{command::Command, context::CliCtx};

fn main() -> Result<()> {
    let mut ctx = CliCtx::new();

    match ctx.args.command.clone() {
        Command::Invoke { program_id } => {
            command::invoke::run(&mut ctx, &program_id)?;
        }
    }

    Ok(())
}
