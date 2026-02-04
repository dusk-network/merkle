// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{ARITY, Opening};

use dusk_merkle::Aggregate;
use dusk_plonk::prelude::{BlsScalar, Composer, Constraint, Witness};
use dusk_poseidon::{Domain, HashGadget};

/// Builds the gadget for the poseidon opening and returns the computed
/// root.
pub fn opening_gadget<T, const H: usize>(
    composer: &mut Composer,
    opening: &Opening<T, H>,
    leaf: Witness,
) -> Witness
where
    T: Clone + Aggregate<ARITY>,
{
    // append the siblings and position to the circuit
    let mut level_witnesses = [[Composer::ZERO; ARITY]; H];
    // if i == position: pos_bits[i] = 1 else: pos_bits[i] = 0
    let mut pos_bits = [[Composer::ZERO; ARITY]; H];
    for h in (0..H).rev() {
        let level = &opening.branch()[h];
        for (i, item) in level.iter().enumerate() {
            if i == opening.positions()[h] {
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
        for i in 0..ARITY {
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
        needle =
            HashGadget::digest(composer, Domain::Merkle4, &level_witnesses[h])
                [0];
    }

    // return the computed root as a witness in the circuit
    needle
}
