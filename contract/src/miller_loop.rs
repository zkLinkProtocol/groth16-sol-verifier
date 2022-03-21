use std::slice::Iter;

use ark_bn254::{Fq12Parameters, G1Affine, G1Projective, Parameters};
use ark_ec::bn::{BnParameters, G1Prepared};
use ark_ec::ProjectiveCurve;
use ark_ff::{Field, Fp12, Fp12ParamsWrapper, FromBytes, QuadExtField};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;

use crate::pvk::{get_delta_qef, get_gamma_qef};
use crate::utils::{get_account_data, put_account_data};

pub fn gamma_miller_loop(
    accounts_iter: &mut Iter<AccountInfo>,
    i: usize,
    j: usize,
    input: &[u8],
) -> ProgramResult {
    let gamma_account = next_account_info(accounts_iter)?;

    let prepared_input = G1Projective::read(&mut input.as_ref())
        .unwrap()
        .into_affine()
        .into();
    let account_data = get_account_data(gamma_account, j);
    let account_data = match j {
        89 => final_gamma_miller_loop(&prepared_input, account_data, j),
        _ => sub_gamma_miller_loop(&prepared_input, account_data, i, j),
    };
    put_account_data(gamma_account, &account_data);
    Ok(())
}

pub fn gamma_onchain_ell(f: &mut Fp12<Fq12Parameters>, j: usize, p: &G1Affine) {
    let mut c0 = get_gamma_qef(j, 0);
    let mut c1 = get_gamma_qef(j, 1);
    let c2 = get_gamma_qef(j, 2);

    c0.mul_assign_by_fp(&p.y);
    c1.mul_assign_by_fp(&p.x);
    f.mul_by_034(&c0, &c1, &c2);
}

fn sub_gamma_miller_loop(
    p: &G1Prepared<ark_bn254::Parameters>,
    mut f: QuadExtField<Fp12ParamsWrapper<Fq12Parameters>>,
    i: usize,
    j: usize,
) -> QuadExtField<Fp12ParamsWrapper<Fq12Parameters>> {
    if !p.is_zero() {
        if i != ark_bn254::Parameters::ATE_LOOP_COUNT.len() - 1 {
            f.square_in_place();
        }
        gamma_onchain_ell(&mut f, j, &p.0);
        match ark_bn254::Parameters::ATE_LOOP_COUNT[i - 1] {
            1 => {
                gamma_onchain_ell(&mut f, j + 1, &p.0);
            }
            -1 => {
                gamma_onchain_ell(&mut f, j + 1, &p.0);
            }
            _ => {}
        }
    }
    f
}

fn final_gamma_miller_loop(
    p: &G1Prepared<ark_bn254::Parameters>,
    mut f: QuadExtField<Fp12ParamsWrapper<Fq12Parameters>>,
    j: usize,
) -> QuadExtField<Fp12ParamsWrapper<Fq12Parameters>> {
    if !p.is_zero() {
        gamma_onchain_ell(&mut f, j, &p.0);
        gamma_onchain_ell(&mut f, j + 1, &p.0);
    }
    f
}

pub fn delta_miller_loop(
    accounts_iter: &mut Iter<AccountInfo>,
    i: usize,
    j: usize,
    input: &[u8],
) -> ProgramResult {
    let delta_account = next_account_info(accounts_iter)?;

    let proof_c = G1Affine::read(&mut input.as_ref())
        .map(|p| G1Prepared::<Parameters>::from(p))
        .unwrap();
    let account_data = get_account_data(delta_account, j);
    let account_data = match j {
        89 => final_delta_miller_loop(&proof_c, account_data, j),
        _ => sub_delta_miller_loop(&proof_c, account_data, i, j),
    };
    put_account_data(delta_account, &account_data);
    Ok(())
}

fn delta_onchain_ell(f: &mut Fp12<Fq12Parameters>, j: usize, p: &G1Affine) {
    let mut c0 = get_delta_qef(j, 0);
    let mut c1 = get_delta_qef(j, 1);
    let c2 = get_delta_qef(j, 2);

    c0.mul_assign_by_fp(&p.y);
    c1.mul_assign_by_fp(&p.x);
    f.mul_by_034(&c0, &c1, &c2);
}

fn sub_delta_miller_loop(
    p: &G1Prepared<ark_bn254::Parameters>,
    mut f: QuadExtField<Fp12ParamsWrapper<Fq12Parameters>>,
    i: usize,
    j: usize,
) -> QuadExtField<Fp12ParamsWrapper<Fq12Parameters>> {
    if !p.is_zero() {
        if i != ark_bn254::Parameters::ATE_LOOP_COUNT.len() - 1 {
            f.square_in_place();
        }
        delta_onchain_ell(&mut f, j, &p.0);
        match ark_bn254::Parameters::ATE_LOOP_COUNT[i - 1] {
            1 => {
                delta_onchain_ell(&mut f, j + 1, &p.0);
            }
            -1 => {
                delta_onchain_ell(&mut f, j + 1, &p.0);
            }
            _ => {}
        }
    }
    f
}

fn final_delta_miller_loop(
    p: &G1Prepared<ark_bn254::Parameters>,
    mut f: QuadExtField<Fp12ParamsWrapper<Fq12Parameters>>,
    j: usize,
) -> QuadExtField<Fp12ParamsWrapper<Fq12Parameters>> {
    if !p.is_zero() {
        delta_onchain_ell(&mut f, j, &p.0);
        delta_onchain_ell(&mut f, j + 1, &p.0);
    }
    f
}
