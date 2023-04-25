// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use blake3::{Hash, Hasher};

const HEIGHT: usize = 32;
const ARITIES: &[usize] = &[2, 4, 8];

fn main() {
    let mut hashes = [[0u8; 32]; HEIGHT + 1];
    let zero_leaf = *b"Dusk Network -- Defendit Numerus";

    for a in ARITIES {
        let mut empty_hash = Hash::from(zero_leaf);
        for h in (0..HEIGHT + 1).rev() {
            hashes[h] = empty_hash.into();
            let mut hasher = Hasher::new();
            for _ in 0..*a {
                hasher.update(empty_hash.as_bytes());
            }
            empty_hash = hasher.finalize();
        }

        println!("[ARITY: {a}]");
        for h in hashes {
            println!("{}", hex::encode(h));
        }
    }
}
