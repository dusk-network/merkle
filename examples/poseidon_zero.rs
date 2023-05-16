// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::BlsScalar;
use dusk_poseidon::sponge::hash;
use std::mem;

const HEIGHT: usize = 64;
const ARITIES: &[usize] = &[2, 4, 8];

fn main() {
    let mut hashes = [BlsScalar::zero(); HEIGHT + 1];
    let zero_leaf = unsafe {
        BlsScalar::from_raw(mem::transmute(
            *b"Dusk Network -- Defendit Numerus",
        ))
    };

    for a in ARITIES {
        let mut empty_hash = zero_leaf;
        for h in (0..HEIGHT + 1).rev() {
            let level = [empty_hash; 8];
            hashes[h] = empty_hash.into();
            empty_hash = hash(&level[..*a]);
        }

        println!("[ARITY: {a}]");
        for h in hashes {
            println!(
                "BlsScalar([0x{:016x}, 0x{:016x}, 0x{:016x}, 0x{:016x}]),",
                h.0[0], h.0[1], h.0[2], h.0[3]
            );
        }
    }
}
