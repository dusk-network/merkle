// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use blake3::{Hash as Blake3Hash, Hasher};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
#[cfg(feature = "rkyv-impl")]
use criterion::{SamplingMode, black_box};
use dusk_merkle::{Aggregate, Tree};
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

const EMPTY_HASH: Item = Item([0; 32]);

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
pub struct Item([u8; 32]);

impl From<Blake3Hash> for Item {
    fn from(h: Blake3Hash) -> Self {
        Self(h.into())
    }
}

impl<const A: usize> Aggregate<A> for Item {
    const EMPTY_SUBTREE: Self = EMPTY_HASH;

    fn aggregate(items: [&Self; A]) -> Self {
        let mut hasher = Hasher::new();
        for item in items {
            hasher.update(&item.0);
        }
        hasher.finalize().into()
    }
}

impl Item {
    #[must_use]
    pub fn new(bytes: [u8; 32]) -> Self {
        Item(bytes)
    }
}

const H: usize = 32;
const A: usize = 2;

type Blake3Tree = Tree<Item, H, A>;
// Matches Piecrust's 32-bit memory page-tree shape (65,536-page capacity).
#[cfg(feature = "rkyv-impl")]
type DecodedBlake3Tree = Tree<Item, 8, 4>;
// Matches Piecrust's 64-bit memory page-tree shape.
#[cfg(feature = "rkyv-impl")]
type DecodedBlake3Tree64 = Tree<Item, 13, 4>;

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
        name: "sparse_4096",
        leaves: 4096,
        occupancy: Occupancy::Sparse,
    },
    Scenario {
        name: "dense_4096",
        leaves: 4096,
        occupancy: Occupancy::Dense,
    },
    Scenario {
        name: "dense_65536",
        leaves: 65536,
        occupancy: Occupancy::Dense,
    },
];

#[cfg(feature = "rkyv-impl")]
const PAGES_PER_GIB: u64 = (1 << 30) / (64 << 10);

#[cfg(feature = "rkyv-impl")]
const WASM64_FIRST_ROOT_SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "dense_10_gib",
        leaves: 10 * PAGES_PER_GIB,
        occupancy: Occupancy::Dense,
    },
    Scenario {
        name: "scattered_10_gib",
        leaves: 10 * PAGES_PER_GIB,
        occupancy: Occupancy::Sparse,
    },
    Scenario {
        name: "dense_50_gib",
        leaves: 50 * PAGES_PER_GIB,
        occupancy: Occupancy::Dense,
    },
    Scenario {
        name: "scattered_50_gib",
        leaves: 50 * PAGES_PER_GIB,
        occupancy: Occupancy::Sparse,
    },
    Scenario {
        name: "dense_100_gib",
        leaves: 100 * PAGES_PER_GIB,
        occupancy: Occupancy::Dense,
    },
    Scenario {
        name: "scattered_100_gib",
        leaves: 100 * PAGES_PER_GIB,
        occupancy: Occupancy::Sparse,
    },
];

#[cfg(feature = "rkyv-impl")]
fn scenario_position<const H: usize>(scenario: Scenario, index: u64) -> u64 {
    match scenario.occupancy {
        Occupancy::Dense => index,
        // The odd multiplier is a permutation modulo the power-of-two tree
        // capacity, so positions are deterministic, unique, and spread out.
        Occupancy::Sparse => {
            let capacity = 1u64 << (2 * H);
            index.wrapping_mul(0x9e37_79b9) & (capacity - 1)
        }
    }
}

#[cfg(feature = "rkyv-impl")]
fn scenario_item(position: u64, generation: u64) -> Item {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&position.to_le_bytes());
    bytes[8..16].copy_from_slice(&generation.to_le_bytes());
    Hasher::new().update(&bytes).finalize().into()
}

#[cfg(feature = "rkyv-impl")]
fn scenario_tree<const H: usize>(scenario: Scenario) -> Tree<Item, H, 4> {
    let mut tree = Tree::new();
    for index in 0..scenario.leaves {
        let position = scenario_position::<H>(scenario, index);
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
fn update_tree<const H: usize>(
    tree: &mut Tree<Item, H, 4>,
    scenario: Scenario,
    update_count: u64,
    generation: u64,
) {
    let stride = scenario.leaves / update_count;
    for update in 0..update_count {
        let index = update * stride;
        let position = scenario_position::<H>(scenario, index);
        tree.insert(position, scenario_item(position, generation));
    }
}

#[cfg(feature = "rkyv-impl")]
fn bench_blake3_first_root(c: &mut Criterion) {
    use std::time::{Duration, Instant};

    let mut group = c.benchmark_group("blake3_first_root");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(4));

    for &scenario in FIRST_ROOT_SCENARIOS {
        let base = scenario_tree::<8>(scenario);
        let warmed_root = *base.root();

        let mut dirty = base.clone();
        update_tree::<8>(&mut dirty, scenario, 1, 1);
        let dirty_root = *dirty.root();
        assert_ne!(warmed_root, dirty_root, "the update must change the root");
        let archive = rkyv::to_bytes::<_, 4096>(&dirty)
            .expect("scenario tree archiving must succeed");
        let decoded = rkyv::from_bytes::<DecodedBlake3Tree>(&archive)
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
                        let tree =
                            rkyv::from_bytes::<DecodedBlake3Tree>(&archive)
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
                    let mut tree = scenario_tree::<8>(scenario);
                    black_box(*tree.root());
                    let mut generation = 1;
                    b.iter_custom(|iterations| {
                        let mut measured = Duration::ZERO;
                        for _ in 0..iterations {
                            // Updating is setup. The timer covers only root
                            // recomputation along the newly dirtied paths.
                            update_tree::<8>(
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
                        let tree =
                            rkyv::from_bytes::<DecodedBlake3Tree>(&archive)
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
fn bench_blake3_wasm64_first_root(c: &mut Criterion) {
    use std::time::{Duration, Instant};

    let mut group = c.benchmark_group("blake3_wasm64_first_root");
    group
        .sample_size(10)
        .sampling_mode(SamplingMode::Flat)
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_secs(2));

    for &scenario in WASM64_FIRST_ROOT_SCENARIOS {
        // The benchmark materializes page hashes and tree nodes, not the
        // corresponding 10-100 GiB of contract memory.
        let mut tree = scenario_tree::<13>(scenario);
        let warmed_root = *tree.root();
        update_tree::<13>(&mut tree, scenario, 1, 1);
        let dirty_root = *tree.root();
        assert_ne!(warmed_root, dirty_root, "the update must change the root");
        let archive = rkyv::to_bytes::<_, 4096>(&tree)
            .expect("scenario tree archiving must succeed");
        drop(tree);

        let decoded = rkyv::from_bytes::<DecodedBlake3Tree64>(&archive)
            .expect("scenario tree checked deserialization must succeed");
        assert_eq!(
            *decoded.root(),
            dirty_root,
            "both paths must compute the same root"
        );
        drop(decoded);

        group.bench_with_input(
            BenchmarkId::new(scenario.name, "cold_decoded"),
            &scenario,
            |b, _| {
                b.iter_custom(|iterations| {
                    let mut measured = Duration::ZERO;
                    for _ in 0..iterations {
                        // Decode remains setup so this isolates the cache
                        // reconstruction caused by the first root read.
                        let tree =
                            rkyv::from_bytes::<DecodedBlake3Tree64>(&archive)
                                .expect("checked deserialization must succeed");
                        let start = Instant::now();
                        black_box(*tree.root());
                        measured += start.elapsed();
                    }
                    measured
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new(scenario.name, "decode_and_first_root"),
            &scenario,
            |b, _| {
                b.iter_custom(|iterations| {
                    let mut measured = Duration::ZERO;
                    for _ in 0..iterations {
                        let start = Instant::now();
                        let tree =
                            rkyv::from_bytes::<DecodedBlake3Tree64>(&archive)
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

#[cfg(feature = "rkyv-impl")]
criterion_group!(
    benches,
    bench_blake3_insert,
    bench_blake3_root,
    bench_blake3_first_root,
    bench_blake3_wasm64_first_root
);
#[cfg(not(feature = "rkyv-impl"))]
criterion_group!(benches, bench_blake3_insert, bench_blake3_root);
criterion_main!(benches);
