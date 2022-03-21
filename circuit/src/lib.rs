use ark_bn254::{Bn254, Fq12Parameters, Fq2Parameters, Fr, G1Affine};
use ark_ec::bn::{BnParameters, G1Prepared, G2Prepared, TwistType};
use ark_ff::{to_bytes, Field, Fp12, Fp12ParamsWrapper, Fp2, QuadExtField};
use ark_groth16::{
    create_random_proof, generate_random_parameters, prepare_inputs, prepare_verifying_key,
};
use ark_relations::r1cs::Result as R1CSResult;
use ark_std::rand;
use ark_std::rand::Rng;
use num_traits::One;

use crate::circuit::{mimc, Circuit, MIMC_ROUNDS};

mod circuit;

pub(crate) type EllCoeff<F> = (F, F, F);

pub fn ell(f: &mut Fp12<Fq12Parameters>, coeffs: &EllCoeff<Fp2<Fq2Parameters>>, p: &G1Affine) {
    let mut c0 = coeffs.0;
    let mut c1 = coeffs.1;
    let mut c2 = coeffs.2;

    match ark_bn254::Parameters::TWIST_TYPE {
        TwistType::M => {
            c2.mul_assign_by_fp(&p.y);
            c1.mul_assign_by_fp(&p.x);
            f.mul_by_014(&c0, &c1, &c2);
        }
        TwistType::D => {
            c0.mul_assign_by_fp(&p.y);
            c1.mul_assign_by_fp(&p.x);
            f.mul_by_034(&c0, &c1, &c2);
        }
    }
}

pub fn initialize() -> R1CSResult<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let rng = &mut {
        use rand::SeedableRng;
        // arbitrary seed
        let seed = [
            1, 0, 0, 0, 23, 0, 0, 0, 200, 1, 0, 0, 210, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        ];
        rand::rngs::StdRng::from_seed(seed)
    };

    let constants = (0..MIMC_ROUNDS).map(|_| rng.gen()).collect::<Vec<_>>();

    println!("Creating parameters...");

    // Create parameters for our circuit
    let params = {
        let c = Circuit::<Fr> {
            xl: None,
            xr: None,
            constants: &constants,
        };

        generate_random_parameters::<Bn254, _, _>(c, rng).unwrap()
    };

    let pvk = prepare_verifying_key(&params.vk);
    println!("Creating proofs...");
    // Generate a random preimage and compute the image
    let l = rng.gen();
    let r = rng.gen();
    let public_inputs = mimc(l, r, &constants);

    // proof_vec.truncate(0);

    // Create an instance of our circuit (with the
    // witness)
    let c = Circuit {
        xl: Some(l),
        xr: Some(r),
        constants: &constants,
    };

    // Create a groth16 proof with our parameters.
    let proof = create_random_proof(c, &params, rng).unwrap();
    let prepared_input = prepare_inputs(&pvk, &[public_inputs])?;
    let mut qap = Fp12::<Fq12Parameters>::one();
    let r = offline_miller_loop(
        &G1Prepared::<ark_bn254::Parameters>::from(proof.a.clone()),
        &G2Prepared::<ark_bn254::Parameters>::from(proof.b.clone()),
        Fp12::<Fq12Parameters>::one(),
    );
    qap *= r;
    Ok((
        to_bytes!(proof.c).unwrap(),
        to_bytes!(prepared_input).unwrap(),
        to_bytes!(qap).unwrap(),
    ))
}

fn offline_miller_loop(
    p: &G1Prepared<ark_bn254::Parameters>,
    q: &G2Prepared<ark_bn254::Parameters>,
    mut f: QuadExtField<Fp12ParamsWrapper<Fq12Parameters>>,
) -> QuadExtField<Fp12ParamsWrapper<Fq12Parameters>> {
    if !p.is_zero() && !q.is_zero() {
        let mut j = 0;
        let coeffs = q.ell_coeffs.as_slice();
        for i in (1..ark_bn254::Parameters::ATE_LOOP_COUNT.len()).rev() {
            if i != ark_bn254::Parameters::ATE_LOOP_COUNT.len() - 1 {
                f.square_in_place();
            }
            ell(&mut f, &coeffs[j], &p.0);
            j += 1;
            match ark_bn254::Parameters::ATE_LOOP_COUNT[i - 1] {
                1 => {
                    ell(&mut f, &coeffs[j], &p.0);
                    j += 1;
                }
                -1 => {
                    ell(&mut f, &coeffs[j], &p.0);
                    j += 1;
                }
                _ => continue,
            }
        }
        ell(&mut f, &coeffs[j], &p.0);
        j += 1;
        ell(&mut f, &coeffs[j], &p.0);
    }
    f
}

#[cfg(test)]
mod tests {
    use crate::initialize;

    #[test]
    fn it_works() {
        println!("{:?}", initialize());
    }
}
