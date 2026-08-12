// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "rkyv-impl")]
use criterion::BenchmarkId;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dusk_bls12_381::BlsScalar;
use dusk_poseidon::{Domain, Hash};
use poseidon_merkle::{Item, Tree};
use rand::{RngCore, SeedableRng};

const HEIGHT: usize = 17;

type PoseidonTree = Tree<(), HEIGHT>;
type PoseidonItem = Item<()>;

fn bench_poseidon(c: &mut Criterion) {
    let tree = &mut PoseidonTree::new();
    let rng = &mut rand::rngs::StdRng::seed_from_u64(0xbeef);

    c.bench_function("poseidon insertion", |b| {
        b.iter(|| {
            let pos = rng.next_u64() % u32::MAX as u64;

            let hash = Hash::digest(Domain::Other, &[BlsScalar::from(pos)])
                .into_iter()
                .next()
                .expect(
                    "Poseidon hash output must contain at least one element",
                );

            let item = PoseidonItem { hash, data: () };
            tree.insert(black_box(pos), black_box(item));
        })
    });
}

#[cfg(feature = "rkyv-impl")]
#[derive(Clone, Copy)]
enum Occupancy {
    Dense,
    Sparse,
}

#[cfg(feature = "rkyv-impl")]
#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    leaves: u64,
    occupancy: Occupancy,
}

#[cfg(feature = "rkyv-impl")]
const FIRST_ROOT_SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "sparse_64",
        leaves: 64,
        occupancy: Occupancy::Sparse,
    },
    Scenario {
        name: "sparse_512",
        leaves: 512,
        occupancy: Occupancy::Sparse,
    },
    Scenario {
        name: "dense_512",
        leaves: 512,
        occupancy: Occupancy::Dense,
    },
    Scenario {
        name: "dense_4096",
        leaves: 4096,
        occupancy: Occupancy::Dense,
    },
];

#[cfg(feature = "rkyv-impl")]
fn scenario_position(scenario: Scenario, index: u64) -> u64 {
    match scenario.occupancy {
        Occupancy::Dense => index,
        // The odd multiplier is a permutation modulo the power-of-two tree
        // capacity, so positions are deterministic, unique, and spread out.
        Occupancy::Sparse => {
            index.wrapping_mul(0x2_78dd_e6e5) & ((1u64 << 34) - 1)
        }
    }
}

#[cfg(feature = "rkyv-impl")]
fn scenario_item(position: u64, generation: u64) -> PoseidonItem {
    let value = position
        .wrapping_add(generation.wrapping_mul(0x9e37_79b9))
        .wrapping_add(1);
    PoseidonItem::new(BlsScalar::from(value), ())
}

#[cfg(feature = "rkyv-impl")]
fn scenario_tree(scenario: Scenario) -> PoseidonTree {
    let mut tree = PoseidonTree::new();
    for index in 0..scenario.leaves {
        let position = scenario_position(scenario, index);
        tree.insert(position, scenario_item(position, 0));
    }
    assert_eq!(
        tree.len(),
        scenario.leaves,
        "scenario positions must be unique"
    );
    tree
}

#[cfg(feature = "rkyv-impl")]
fn update_tree(
    tree: &mut PoseidonTree,
    scenario: Scenario,
    update_count: u64,
    generation: u64,
) {
    let stride = scenario.leaves / update_count;
    for update in 0..update_count {
        let index = update * stride;
        let position = scenario_position(scenario, index);
        tree.insert(position, scenario_item(position, generation));
    }
}

#[cfg(feature = "rkyv-impl")]
fn bench_poseidon_first_root(c: &mut Criterion) {
    use std::time::{Duration, Instant};

    let mut group = c.benchmark_group("poseidon_first_root");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(4));

    for &scenario in FIRST_ROOT_SCENARIOS {
        let base = scenario_tree(scenario);
        let warmed_root = *base.root();

        let mut dirty = base.clone();
        update_tree(&mut dirty, scenario, 1, 1);
        let dirty_root = *dirty.root();
        assert_ne!(warmed_root, dirty_root, "the update must change the root");
        let archive = rkyv::to_bytes::<_, 4096>(&dirty)
            .expect("scenario tree archiving must succeed");
        let decoded = rkyv::from_bytes::<PoseidonTree>(&archive)
            .expect("scenario tree checked deserialization must succeed");
        let decoded_root = *decoded.root();
        assert_eq!(
            decoded_root, dirty_root,
            "both paths must compute the same root"
        );

        group.bench_with_input(
            BenchmarkId::new(scenario.name, "cold_decoded"),
            &scenario,
            |b, _| {
                b.iter_custom(|iterations| {
                    let mut measured = Duration::ZERO;
                    for _ in 0..iterations {
                        // Checked decode is setup: every timed root receives a
                        // fresh tree whose internal caches were discarded.
                        let tree = rkyv::from_bytes::<PoseidonTree>(&archive)
                            .expect("checked deserialization must succeed");
                        let start = Instant::now();
                        black_box(*tree.root());
                        measured += start.elapsed();
                    }
                    measured
                });
            },
        );

        for update_count in [1, 4] {
            group.bench_with_input(
                BenchmarkId::new(
                    scenario.name,
                    format!("warm_dirty_{update_count}"),
                ),
                &scenario,
                |b, &scenario| {
                    let mut tree = scenario_tree(scenario);
                    black_box(*tree.root());
                    let mut generation = 1;
                    b.iter_custom(|iterations| {
                        let mut measured = Duration::ZERO;
                        for _ in 0..iterations {
                            // Updating is setup. The timer covers only root
                            // recomputation along the newly dirtied paths.
                            update_tree(
                                &mut tree,
                                scenario,
                                update_count,
                                generation,
                            );
                            generation += 1;
                            let start = Instant::now();
                            black_box(*tree.root());
                            measured += start.elapsed();
                        }
                        measured
                    });
                },
            );
        }

        group.bench_with_input(
            BenchmarkId::new(scenario.name, "decode_and_first_root"),
            &scenario,
            |b, _| {
                b.iter_custom(|iterations| {
                    let mut measured = Duration::ZERO;
                    for _ in 0..iterations {
                        let start = Instant::now();
                        let tree = rkyv::from_bytes::<PoseidonTree>(&archive)
                            .expect("checked deserialization must succeed");
                        black_box(*tree.root());
                        measured += start.elapsed();
                    }
                    measured
                });
            },
        );
    }
}

#[cfg(feature = "rkyv-impl")]
criterion_group!(benches, bench_poseidon, bench_poseidon_first_root);
#[cfg(not(feature = "rkyv-impl"))]
criterion_group!(benches, bench_poseidon);
criterion_main!(benches);
