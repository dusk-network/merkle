// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::poseidon::Opening;
use crate::Aggregate;

use dusk_plonk::prelude::{BlsScalar, Composer, Constraint, Witness};
use dusk_poseidon::sponge::merkle::gadget as poseidon_merkle_gadget;

impl<T, const H: usize, const A: usize> Opening<T, H, A>
where
    T: Clone + Aggregate<A>,
{
    /// Builds the gadget for the poseidon opening and returns the computed
    /// root.
    pub fn gadget<C>(&self, composer: &mut C, leaf: Witness) -> Witness
    where
        C: Composer,
    {
        // append the siblings and position to the circuit
        let mut level_witnesses = [[C::ZERO; A]; H];
        // if i == position: pos_bits[i] = 1 else: pos_bits[i] = 0
        let mut pos_bits = [[C::ZERO; A]; H];
        for h in (0..H).rev() {
            let level = &self.branch()[h];
            for (i, item) in level.iter().enumerate() {
                if i == self.positions()[h] {
                    pos_bits[h][i] = composer.append_witness(BlsScalar::one());
                } else {
                    pos_bits[h][i] = composer.append_witness(BlsScalar::zero());
                }

                level_witnesses[h][i] = composer.append_witness(item.hash);
                // ensure that the entries of pos_bits are either 0 or 1
                composer.component_boolean(pos_bits[h][i]);
            }

            // ensure there is *exactly* one bit turned on in the array, by
            // checking that the sum of all position bits equals 1
            let constraint = Constraint::new()
                .left(1)
                .a(pos_bits[h][0])
                .right(1)
                .b(pos_bits[h][1])
                .fourth(1)
                .d(pos_bits[h][2]);
            let mut sum = composer.gate_add(constraint);
            let constraint =
                Constraint::new().left(1).a(sum).right(1).b(pos_bits[h][3]);
            sum = composer.gate_add(constraint);
            composer.assert_equal_constant(sum, BlsScalar::one(), None);
        }

        // keep track of the computed hash along our path with needle
        let mut needle = leaf;
        for h in (0..H).rev() {
            for i in 0..A {
                // assert that:
                // pos_bits[h][i] * level_hash[i] = pos_bits[h][i] * needle
                let constraint = Constraint::new()
                    .mult(1)
                    .a(pos_bits[h][i])
                    .b(level_witnesses[h][i]);
                let result = composer.gate_mul(constraint);
                let constraint =
                    Constraint::new().mult(1).a(pos_bits[h][i]).b(needle);
                let needle_result = composer.gate_mul(constraint);
                // ensure the computed hash matches the stored one
                composer.assert_equal(result, needle_result);
            }

            // hash the current level
            needle = poseidon_merkle_gadget(composer, &level_witnesses[h]);
        }

        // return the computed root as a witness in the circuit
        needle
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use core::cmp;

    use dusk_plonk::prelude::{
        Circuit, Compiler, Composer, Error, PublicParameters,
    };
    use dusk_poseidon::sponge::merkle::hash as poseidon_merkle;
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};

    use crate::poseidon::Item;
    use crate::poseidon::Tree;

    // set max circuit size to 2^15 gates
    const CAPACITY: usize = 15;

    // set height and arity of the poseidon merkle tree
    const HEIGHT: usize = 17;
    const ARITY: usize = 4;

    type PoseidonItem = Item<Option<BHRange>>;

    // block-height range type keeps track of the min and max block height
    // of all children
    #[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
    struct BHRange {
        min: u64,
        max: u64,
    }

    // implement Aggregate for BHRange type
    impl Aggregate<ARITY> for Option<BHRange> {
        const EMPTY_SUBTREE: Self = None;

        fn aggregate(items: [&Self; ARITY]) -> Self {
            let mut bh_range = None;
            for item in items {
                bh_range = match (bh_range, item.as_ref()) {
                    (None, None) => None,
                    (None, Some(r)) => Some(*r),
                    (Some(r), None) => Some(r),
                    (Some(bh_range), Some(item_bh_range)) => {
                        let min = cmp::min(item_bh_range.min, bh_range.min);
                        let max = cmp::max(item_bh_range.max, bh_range.max);
                        Some(BHRange { min, max })
                    }
                };
            }
            bh_range
        }
    }

    // Create a circuit for the opening
    #[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
    struct OpeningCircuit {
        opening: Opening<Option<BHRange>, HEIGHT, ARITY>,
        leaf: PoseidonItem,
    }

    impl Default for OpeningCircuit {
        fn default() -> Self {
            let mut tree = Tree::new();
            tree.insert(0, PoseidonItem::EMPTY_SUBTREE);
            let opening =
                tree.opening(0).expect("There is a leaf at position 0");
            Self {
                opening,
                leaf: PoseidonItem::EMPTY_SUBTREE,
            }
        }
    }

    impl OpeningCircuit {
        /// Create a new OpeningCircuit
        pub fn new(
            opening: Opening<Option<BHRange>, HEIGHT, ARITY>,
            leaf: PoseidonItem,
        ) -> Self {
            Self { opening, leaf }
        }
    }

    impl Circuit for OpeningCircuit {
        fn circuit<C>(&self, composer: &mut C) -> Result<(), Error>
        where
            C: Composer,
        {
            // append the leaf and opening gadget to the circuit
            let leaf = composer.append_witness(self.leaf.hash);
            let computed_root = self.opening.gadget(composer, leaf);

            // append the public root as public input to the circuit
            // and ensure it is equal to the computed root
            let constraint = Constraint::new()
                .left(-BlsScalar::one())
                .a(computed_root)
                .public(self.opening.root().hash);
            composer.append_gate(constraint);

            Ok(())
        }
    }

    #[test]
    fn opening() {
        let label = b"merkle opening";
        let rng = &mut StdRng::seed_from_u64(0xdea1);
        let pp = PublicParameters::setup(1 << CAPACITY, rng).unwrap();

        let (prover, verifier) =
            Compiler::compile::<OpeningCircuit>(&pp, label)
                .expect("Circuit should compile successfully");

        let mut tree = Tree::new();
        let mut leaf = PoseidonItem::new(BlsScalar::zero(), None);
        let mut position = 0;
        for bh in 0..100 {
            let hash = poseidon_merkle(&[BlsScalar::random(rng)]);
            position = rng.next_u64() % u8::MAX as u64;
            leaf = PoseidonItem::new(hash, Some(BHRange { min: bh, max: bh }));
            tree.insert(position as u64, leaf);
        }
        let opening = tree.opening(position as u64).unwrap();
        assert!(opening.verify(leaf.clone()));

        let circuit = OpeningCircuit::new(opening, leaf);

        let (proof, public_inputs) = prover
            .prove(rng, &circuit)
            .expect("Proof generation should succeed");

        verifier
            .verify(&proof, &public_inputs)
            .expect("Proof verification should succeed");
    }
}
