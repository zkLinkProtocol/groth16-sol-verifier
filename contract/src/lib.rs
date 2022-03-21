use crate::final_exponentiation::final_exponentiation;
use crate::miller_loop::{delta_miller_loop, gamma_miller_loop};
use crate::utils::unpack_instruction_data;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

mod final_exponentiation;
mod miller_loop;
mod pvk;
mod utils;

entrypoint!(process_instruction);
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let (t, i, j, input) = unpack_instruction_data(instruction_data).unwrap();
    match t {
        0 => gamma_miller_loop(accounts_iter, i, j, input),
        1 => delta_miller_loop(accounts_iter, i, j, input),
        _ => final_exponentiation(accounts_iter, i, j, input),
    }
}
