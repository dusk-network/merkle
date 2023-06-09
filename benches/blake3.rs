// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

use blake3::Hasher;

use dusk_merkle::blake3::Item;
use dusk_merkle::Tree;

const H: usize = 32;
const A: usize = 2;

type Blake3Tree = Tree<Item, H, A>;

const NS: &[u64] = &[10, 100, 1000, 10000];

fn bench_blake3_insert(c: &mut Criterion) {
    let rng = &mut StdRng::seed_from_u64(0xbeef);

    let mut group = c.benchmark_group("blake3_insert_n");
    for n in NS {
        group.bench_with_input(
            BenchmarkId::from_parameter(*n),
            n,
            |b, &size| {
                b.iter(|| {
                    let mut tree = Blake3Tree::new();
                    insert_random_n(rng, &mut tree, size);
                });
            },
        );
    }
}

fn bench_blake3_root(c: &mut Criterion) {
    let rng = &mut StdRng::seed_from_u64(0xbeef);

    let mut group = c.benchmark_group("blake3_root_n");
    for n in NS {
        group.bench_with_input(
            BenchmarkId::from_parameter(*n),
            n,
            |b, &size| {
                b.iter(|| {
                    let mut tree = Blake3Tree::new();
                    insert_random_n(rng, &mut tree, size);
                    let _root = *tree.root();
                });
            },
        );
    }
}

fn insert_random_n<Rng: RngCore>(rng: &mut Rng, tree: &mut Blake3Tree, n: u64) {
    let cap = tree.capacity();

    let mut hash_bytes = [0u8; 32];
    rng.fill_bytes(&mut hash_bytes);
    let mut hasher = Hasher::new();
    hasher.update(&hash_bytes);
    let hash: Item = hasher.finalize().into();

    for _ in 0..n {
        let pos = rng.next_u64() % cap;
        tree.insert(pos, hash);
    }
}

criterion_group!(benches, bench_blake3_insert, bench_blake3_root);
criterion_main!(benches);
