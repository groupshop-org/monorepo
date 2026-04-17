use pinocchio::{entrypoint, AccountView, Address, ProgramResult};

pub mod errors;
pub mod instruction;
pub mod pda;
pub mod processor;
pub mod state;

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process(program_id, accounts, instruction_data)
}
