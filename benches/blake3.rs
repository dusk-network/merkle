// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// #![cfg(features = "poseidon")]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use rand::{RngCore, SeedableRng};

use blake3::Hasher;
use dusk_merkle::blake3::Item;
use dusk_merkle::Tree;

const H: usize = 32;
const A: usize = 4;

type Blake3Tree = Tree<Item, H, A>;

fn bench_blake3(c: &mut Criterion) {
    let tree = &mut Blake3Tree::new();
    let rng = &mut rand::rngs::StdRng::seed_from_u64(0xbeef);

    c.bench_function("blake3 insertion", |b| {
        b.iter(|| {
            let pos = rng.next_u64();

            let mut hash_bytes = [0u8; 32];
            rng.fill_bytes(&mut hash_bytes);
            let mut hasher = Hasher::new();
            hasher.update(&hash_bytes);
            let hash: Item = hasher.finalize().into();

            tree.insert(black_box(pos), black_box(hash));
        })
    });
}

criterion_group!(benches, bench_blake3);
criterion_main!(benches);
