use ark_bn254::Fq12Parameters;
use ark_ff::{to_bytes, Fp12, Fp12ParamsWrapper, FromBytes, QuadExtField};
use arrayref::{array_mut_ref, array_ref};
use num_traits::One;
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;

pub const BN254_DATA_LEN: usize = 384;

pub fn unpack_instruction_data(input: &[u8]) -> Result<(usize, usize, usize, &[u8]), ProgramError> {
    let (&t, rest) = input
        .split_first()
        .ok_or(solana_program::program_error::INVALID_INSTRUCTION_DATA)?;
    let (&i, rest) = rest
        .split_first()
        .ok_or(solana_program::program_error::INVALID_INSTRUCTION_DATA)?;
    let (&j, rest) = rest
        .split_first()
        .ok_or(solana_program::program_error::INVALID_INSTRUCTION_DATA)?;
    Ok((t as usize, i as usize, j as usize, rest))
}

pub fn get_account_data(
    account: &AccountInfo,
    j: usize,
) -> QuadExtField<Fp12ParamsWrapper<Fq12Parameters>> {
    let f = match j {
        0 => Fp12::<Fq12Parameters>::one(),
        _ => {
            let src = account.try_borrow_data().unwrap();
            let src = array_ref![src, 0, 384];
            Fp12::<Fq12Parameters>::read(&mut src.as_ref()).unwrap()
        }
    };
    f
}

pub fn put_account_data(
    account: &AccountInfo,
    f: &QuadExtField<Fp12ParamsWrapper<Fq12Parameters>>,
) {
    let mut dst = account.try_borrow_mut_data().unwrap();
    let dst = array_mut_ref![dst, 0, BN254_DATA_LEN];
    dst.copy_from_slice(to_bytes!(f).unwrap().as_slice());
}
