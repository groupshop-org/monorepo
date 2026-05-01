mod command;
mod context;

use anyhow::Result;

use crate::{command::Command, context::CliCtx};

#[tokio::main]
async fn main() -> Result<()> {
    let mut ctx = CliCtx::new();

    match ctx.args.command.clone() {
        Command::Invoke { program_id } => {
            command::invoke::run(&mut ctx, &program_id)?;
        }
        Command::Import {
            csv,
            api_url,
            max_per_category,
            min_moq,
            dry_run,
        } => {
            command::import::run(
                &csv,
                &api_url,
                &ctx.args.api_auth_email,
                &ctx.args.api_auth_password,
                max_per_category,
                min_moq,
                dry_run,
            )
            .await?;
        }
        Command::WipeCatalog { api_url } => {
            command::wipe_catalog::run(
                &api_url,
                &ctx.args.api_auth_email,
                &ctx.args.api_auth_password,
            )
            .await?;
        }
        Command::Market { sub } => {
            command::market::run(&mut ctx, sub)?;
        }
    }

    Ok(())
}
