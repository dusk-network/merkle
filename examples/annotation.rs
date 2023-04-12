// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp;
use std::ops::Range;
use std::time::Instant;

use blake3::{Hash, Hasher};
use dusk_merkle::{Aggregate, Tree as MerkleTree};

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

const H: usize = 17;
const A: usize = 4;

struct Annotation {
    hash: Hash,
    bh_range: Option<Range<u64>>,
}

impl Aggregate for Annotation {
    const NULL: Self = Self {
        hash: Hash::from_bytes([0u8; 32]),
        bh_range: None,
    };

    fn aggregate<'a, I>(items: I) -> Self
    where
        Self: 'a,
        I: ExactSizeIterator<Item = &'a Self>,
    {
        let mut hasher = Hasher::new();
        let mut bh_range = None;

        for item in items {
            hasher.update(item.hash.as_bytes());

            bh_range = match (bh_range, item.bh_range.as_ref()) {
                (None, None) => None,
                (None, Some(r)) => Some(r.clone()),
                (Some(r), None) => Some(r.clone()),
                (Some(bh_range), Some(item_bh_range)) => {
                    let start = cmp::min(item_bh_range.start, bh_range.start);
                    let end = cmp::max(item_bh_range.end, bh_range.end);
                    Some(start..end)
                }
            };
        }

        Self {
            hash: hasher.finalize(),
            bh_range,
        }
    }
}

struct Note {
    value: u64,
    pk: [u8; 32],
}

impl From<(Note, u64)> for Annotation {
    fn from((note, block_height): (Note, u64)) -> Self {
        let value_bytes = note.value.to_le_bytes();

        let mut hasher = Hasher::new();

        hasher.update(&value_bytes);
        hasher.update(&note.pk);

        Self {
            hash: hasher.finalize(),
            bh_range: Some(block_height..block_height),
        }
    }
}

type Tree = MerkleTree<Annotation, H, A>;

fn main() {
    let tree = &mut Tree::new();
    let rng = &mut StdRng::seed_from_u64(0xbeef);

    const PK: [u8; 32] = [42u8; 32];
    const NOTES_NUM: usize = 1000000;

    let now = Instant::now();

    for _ in 0..NOTES_NUM {
        let note = Note {
            pk: PK,
            value: rng.next_u64(),
        };

        let block_height = rng.next_u64() % 1000;

        let pos = rng.next_u64() % tree.capacity();
        tree.insert(pos, (note, block_height));
    }

    let elapsed = now.elapsed();
    println!("Took {}ms to insert {NOTES_NUM} items", elapsed.as_millis());
}
