use algebra::sw6::Fr as SW6Fr;
use algebra::test_rng;
use r1cs_core::ConstraintSystem;
use r1cs_std::bls12_377::G1Gadget;
use r1cs_std::prelude::{AllocGadget, UInt8};
use r1cs_std::test_constraint_system::TestConstraintSystem;
use rand::RngCore;

use nano_sync::constants::sum_generator_g1;
use nano_sync::gadgets::PedersenHashGadget;
use nano_sync::primitives::{evaluate_pedersen, setup_pedersen};

#[test]
fn pedersen_test() {
    // Initialize the constraint system.
    let mut cs = TestConstraintSystem::<SW6Fr>::new();

    // Create random bytes.
    let rng = &mut test_rng();
    let mut input = [0u8; 32];
    rng.fill_bytes(&mut input);

    // Generate the generators for the Pedersen hash.
    let generators = setup_pedersen();

    // Evaluate Pedersen hash using the primitive version.
    let primitive_out = evaluate_pedersen(generators.clone(), input.to_vec(), sum_generator_g1());

    // Convert the result to a G1Gadget for easier comparison.
    let primitive_out_var = G1Gadget::alloc(cs.ns(|| "prim g1"), || Ok(primitive_out)).unwrap();

    // Allocate the random bytes in the circuit.
    let mut input_bytes = vec![];
    for (byte_i, input_byte) in input.iter().enumerate() {
        input_bytes.push(
            UInt8::alloc(cs.ns(|| format!("input_byte_gadget_{}", byte_i)), || {
                Ok(*input_byte)
            })
            .unwrap(),
        );
    }

    // Allocate the Pedersen generators in the circuit.
    let mut c_generators = Vec::new();
    for i in 0..generators.len() {
        let base = G1Gadget::alloc(&mut cs.ns(|| format!("alloc gen {}", i)), || {
            Ok(&generators[i])
        })
        .unwrap();
        c_generators.push(base);
    }

    // Allocate the sum generator in the circuit.
    let sum_generator =
        G1Gadget::alloc(cs.ns(|| "sum generator g1"), || Ok(sum_generator_g1())).unwrap();

    // Evaluate Pedersen hash using the gadget version.
    let gadget_out = PedersenHashGadget::evaluate(
        cs.ns(|| "evaluate pedersen"),
        &c_generators,
        &input_bytes,
        &sum_generator,
    )
    .unwrap();

    assert_eq!(primitive_out_var, gadget_out)
}