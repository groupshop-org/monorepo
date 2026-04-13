use std::str::FromStr;

use anyhow::{Context, Result};
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signer::Signer, transaction::Transaction,
};

use crate::context::CliCtx;

pub fn run(ctx: &mut CliCtx, program_id_str: &str) -> Result<()> {
    let program_id = Pubkey::from_str(program_id_str).context("invalid program ID")?;
    let payer_pubkey = ctx.keypair()?.pubkey();

    // Build a minimal instruction (no accounts, no data)
    let ix = Instruction::new_with_bytes(program_id, &[], vec![]);
    let blockhash = ctx
        .rpc_client()
        .get_latest_blockhash()
        .context("failed to get blockhash")?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer_pubkey),
        &[ctx.keypair()?],
        blockhash,
    );

    // Simulate first to capture logs
    println!("Simulating transaction...");
    let sim = ctx
        .rpc_client()
        .simulate_transaction(&tx)
        .context("simulation failed")?;

    if let Some(logs) = &sim.value.logs {
        println!("\nProgram logs:");
        for log in logs {
            println!("  {log}");
        }
    }

    if let Some(err) = &sim.value.err {
        anyhow::bail!("Simulation error: {err:?}");
    }

    // Send for real
    println!("\nSending transaction...");
    let sig = ctx
        .rpc_client()
        .send_and_confirm_transaction(&tx)
        .context("transaction failed")?;
    println!("Confirmed: {sig}");

    Ok(())
}
