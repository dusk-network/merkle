// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use dusk_merkle::{Aggregate, Opening, Tree};

use ff::Field;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

const H: usize = 17;
const A: usize = 4;

#[derive(Clone, Copy, PartialEq, Debug)]
struct Range {
    start: u64,
    end: u64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Item {
    hash: BlsScalar,
    bh_range: Option<Range>,
}

const EMPTY_ITEM: Item = Item {
    hash: BlsScalar::zero(),
    bh_range: None,
};

impl Aggregate<A> for Item {
    const EMPTY_SUBTREE: Self = EMPTY_ITEM;

    fn aggregate(items: [&Self; A]) -> Self {
        let mut bh_range = None;
        let mut rng = StdRng::seed_from_u64(0xbeef);

        for item in items {
            bh_range = match (bh_range, item.bh_range.as_ref()) {
                (None, None) => None,
                (None, Some(r)) => Some(*r),
                (Some(r), None) => Some(r),
                (Some(bh_range), Some(item_bh_range)) => {
                    let start = cmp::min(item_bh_range.start, bh_range.start);
                    let end = cmp::max(item_bh_range.end, bh_range.end);
                    Some(Range { start, end })
                }
            };
        }

        Self {
            hash: BlsScalar::random(&mut rng),
            bh_range,
        }
    }
}

// scalar + bh_range
// 32     + 2 + 8 + 8
const ITEM_SIZE: usize = 50;

impl Serializable<ITEM_SIZE> for Item {
    type Error = BytesError;

    fn from_bytes(buf: &[u8; ITEM_SIZE]) -> Result<Self, Self::Error> {
        if buf.len() != ITEM_SIZE {
            return Err(BytesError::BadLength {
                found: (buf.len()),
                expected: (ITEM_SIZE),
            });
        }

        let mut bytes = &buf[..];

        // deserialize hash
        let hash = BlsScalar::from_reader(&mut bytes)?;

        // deserialize bh_range
        let bh_range = match u16::from_reader(&mut bytes)? {
            0 => None,
            1 => {
                let start = u64::from_reader(&mut bytes)?;
                let end = u64::from_reader(&mut bytes)?;
                Some(Range { start, end })
            }
            _ => {
                return Err(BytesError::InvalidData);
            }
        };

        Ok(Item { hash, bh_range })
    }

    fn to_bytes(&self) -> [u8; ITEM_SIZE] {
        let mut buf = [0u8; ITEM_SIZE];

        // serialize hash
        buf[0..BlsScalar::SIZE].copy_from_slice(&self.hash.to_bytes());

        // serialize bh_range
        match self.bh_range {
            // the buffer was initialized with zeros so there is nothing to be
            // done for None
            None => {}
            Some(bh_range) => {
                buf[BlsScalar::SIZE..BlsScalar::SIZE + 2]
                    .copy_from_slice(&1u16.to_bytes());
                buf[BlsScalar::SIZE + 2..BlsScalar::SIZE + 10]
                    .copy_from_slice(&bh_range.start.to_bytes());
                buf[BlsScalar::SIZE + 10..]
                    .copy_from_slice(&bh_range.end.to_bytes());
            }
        };

        buf
    }
}

type MerkleTree = Tree<Item, H, A>;

#[test]
fn serialize_deserialize() {
    let tree = &mut MerkleTree::new();
    let mut rng = StdRng::seed_from_u64(0xbeef);

    const LEAVES: usize = 1000000;

    let mut i = 0;
    let (pos, leaf) = loop {
        i += 1;
        let block_height = rng.next_u64() % 1000;
        let bh_range = Some(Range {
            start: block_height,
            end: block_height,
        });
        let leaf = Item {
            hash: BlsScalar::random(&mut rng),
            bh_range,
        };
        let pos = rng.next_u64() % tree.capacity();
        tree.insert(pos, leaf);
        if i == LEAVES {
            break (pos, leaf);
        }
    };

    let opening = tree.opening(pos).unwrap();

    let serialized = opening.to_var_bytes();
    let deserialized = Opening::<Item, H, A>::from_slice(&serialized).unwrap();

    assert!(deserialized.verify(leaf));
    assert_eq!(opening, deserialized);
}
