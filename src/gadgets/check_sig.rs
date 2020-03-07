use crate::{end_cost_analysis, next_cost_analysis, start_cost_analysis};
use algebra::curves::bls12_377::Bls12_377Parameters;
use algebra::fields::sw6::Fr as SW6Fr;
use r1cs_core::SynthesisError;
use r1cs_std::bits::boolean::Boolean;
use r1cs_std::eq::ConditionalEqGadget;
use r1cs_std::fields::FieldGadget;
use r1cs_std::groups::curves::short_weierstrass::bls12::G1Gadget;
use r1cs_std::groups::curves::short_weierstrass::bls12::G2Gadget;
use r1cs_std::pairing::bls12_377::PairingGadget;
use r1cs_std::pairing::PairingGadget as PG;

pub struct CheckSigGadget {}

impl CheckSigGadget {
    pub fn conditional_check_signature<CS: r1cs_core::ConstraintSystem<SW6Fr>>(
        mut cs: CS,
        public_key: &G2Gadget<Bls12_377Parameters>,
        generator: &G2Gadget<Bls12_377Parameters>,
        signature: &G1Gadget<Bls12_377Parameters>,
        hash_point: &G1Gadget<Bls12_377Parameters>,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        #[allow(unused_mut)]
        let mut cost = start_cost_analysis!(cs, || "Prepare g1 (sig & hash)");
        let sig_p_var = PairingGadget::prepare_g1(cs.ns(|| "sig_p"), &signature)?;
        let hash_p_var = PairingGadget::prepare_g1(cs.ns(|| "hash_p"), &hash_point)?;

        next_cost_analysis!(cs, cost, || "Prepare g2 (generator & pubkey)");
        let generator_p_var = PairingGadget::prepare_g2(cs.ns(|| "generator"), &generator)?;
        let pub_key_p_var = PairingGadget::prepare_g2(cs.ns(|| "pub_key_p"), &public_key)?;

        next_cost_analysis!(cs, cost, || "Pairing 1 (sig & generator)");
        let pairing1_var =
            PairingGadget::pairing(cs.ns(|| "sig pairing"), sig_p_var, generator_p_var.clone())?;
        next_cost_analysis!(cs, cost, || "Pairing 2 (hash & pub)");
        let pairing2_var =
            PairingGadget::pairing(cs.ns(|| "pub pairing"), hash_p_var, pub_key_p_var)?;

        next_cost_analysis!(cs, cost, || "Equality check");
        pairing1_var.conditional_enforce_equal(
            cs.ns(|| "pairing equality"),
            &pairing2_var,
            condition,
        )?;
        end_cost_analysis!(cs, cost);
        Ok(())
    }

    /// Implements signature aggregation from https://crypto.stanford.edu/%7Edabo/pubs/papers/aggreg.pdf .
    pub fn conditional_check_signatures<CS: r1cs_core::ConstraintSystem<SW6Fr>>(
        mut cs: CS,
        public_keys: &[G2Gadget<Bls12_377Parameters>],
        generator: &G2Gadget<Bls12_377Parameters>,
        signature: &G1Gadget<Bls12_377Parameters>,
        hash_points: &[G1Gadget<Bls12_377Parameters>],
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        assert_eq!(
            hash_points.len(),
            public_keys.len(),
            "One hash point per public key is required"
        );
        assert!(hash_points.len() > 1, "Min one message is required");

        #[allow(unused_mut)]
        let mut cost = start_cost_analysis!(cs, || "Prepare g1 sig and g2 generator");
        let sig_p_var = PairingGadget::prepare_g1(cs.ns(|| "sig_p"), &signature)?;
        let generator_p_var = PairingGadget::prepare_g2(cs.ns(|| "generator"), &generator)?;

        next_cost_analysis!(cs, cost, || "Prepare g1 hash points");
        let mut hash_p_vars = vec![];
        for (i, hash_point) in hash_points.iter().enumerate() {
            let hash_p_var =
                PairingGadget::prepare_g1(cs.ns(|| format!("hash_p {}", i)), &hash_point)?;
            hash_p_vars.push(hash_p_var);
        }

        next_cost_analysis!(cs, cost, || "Prepare g2 public keys");
        let mut pub_key_p_vars = vec![];
        for (i, public_key) in public_keys.iter().enumerate() {
            let pub_key_p_var =
                PairingGadget::prepare_g2(cs.ns(|| format!("pub_key_p {}", i)), &public_key)?;
            pub_key_p_vars.push(pub_key_p_var);
        }

        next_cost_analysis!(cs, cost, || "Pairing 1 (sig & generator)");
        let pairing1_var =
            PairingGadget::pairing(cs.ns(|| "sig pairing"), sig_p_var, generator_p_var.clone())?;

        next_cost_analysis!(cs, cost, || "Pairings 2 (hash & pub)");
        let mut pairings2_var = vec![];
        for (i, (hash_p_var, pub_key_p_var)) in hash_p_vars
            .drain(..)
            .zip(pub_key_p_vars.drain(..))
            .enumerate()
        {
            let pairing2_var = PairingGadget::pairing(
                cs.ns(|| format!("pub pairing {}", i)),
                hash_p_var,
                pub_key_p_var,
            )?;
            pairings2_var.push(pairing2_var);
        }

        next_cost_analysis!(cs, cost, || "Add pairings 2");
        let mut pairing2_var = pairings2_var.pop().unwrap();
        for (i, p) in pairings2_var.iter().enumerate() {
            pairing2_var.mul_in_place(cs.ns(|| format!("add pk {}", i)), p)?;
        }

        next_cost_analysis!(cs, cost, || "Equality check");
        pairing1_var.conditional_enforce_equal(
            cs.ns(|| "pairing equality"),
            &pairing2_var,
            condition,
        )?;
        end_cost_analysis!(cs, cost);
        Ok(())
    }
}