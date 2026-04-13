use pinocchio::{entrypoint, AccountView, Address, ProgramResult};
use solana_program_log::log;

// Entry point of the program, called by the Solana runtime when the program is invoked
//
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    log("Hello from my pinocchio program!");
    Ok(())
}
