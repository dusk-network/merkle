// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use dusk_bls12_381::BlsScalar;
use dusk_merkle::poseidon::{Item, Tree};
use dusk_poseidon::sponge::hash as poseidon_hash;

use rand::{RngCore, SeedableRng};

// set height and arity of the poseidon merkle tree
const HEIGHT: usize = 17;
const ARITY: usize = 4;

type PoseidonTree = Tree<(), HEIGHT, ARITY>;
type PoseidonItem = Item<()>;

fn bench_poseidon(c: &mut Criterion) {
    let tree = &mut PoseidonTree::new();
    let rng = &mut rand::rngs::StdRng::seed_from_u64(0xbeef);

    c.bench_function("poseidon insertion", |b| {
        b.iter(|| {
            let pos = rng.next_u64() % u32::MAX as u64;
            let hash = poseidon_hash(&[BlsScalar::from(pos)]);
            let item = PoseidonItem { hash, data: () };
            tree.insert(black_box(pos), black_box(item));
        })
    });
}
criterion_group!(benches, bench_poseidon);
criterion_main!(benches);
