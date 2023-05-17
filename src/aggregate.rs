// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// A type that can be produced by aggregating multiple instances of itself, at
/// certain heights of the tree.
pub trait Aggregate<const H: usize, const A: usize>: Copy {
    /// The items to be used for a given empty subtree at the given height.
    const EMPTY_SUBTREES: [Self; H];

    /// Aggregate the given `items` to produce a single one. The given iterator
    /// is guaranteed to produce `A` number of items, from the leftmost to the
    /// rightmost child of a tree's node.
    fn aggregate<'a, I>(items: I) -> Self
    where
        Self: 'a,
        I: Iterator<Item = &'a Self>;
}

#[cfg(feature = "blake3")]
mod blake {
    use super::Aggregate;
    use blake3::{Hash as Blake3Hash, Hasher};

    const H: usize = 32;
    const A: usize = 4;

    const EMPTY_HASH: Hash = Hash([0; 32]);

    #[derive(Debug, Clone, Copy)]
    struct Hash([u8; 32]);

    impl From<Blake3Hash> for Hash {
        fn from(h: Blake3Hash) -> Self {
            Self(h.into())
        }
    }

    impl Aggregate<H, A> for Hash {
        const EMPTY_SUBTREES: [Self; H] = [EMPTY_HASH; H];

        fn aggregate<'a, I>(items: I) -> Self
        where
            Self: 'a,
            I: Iterator<Item = &'a Self>,
        {
            let mut hasher = Hasher::new();
            for item in items {
                hasher.update(&item.0);
            }
            hasher.finalize().into()
        }
    }

    #[cfg(test)]
    #[cfg(feature = "bench")]
    mod bench {
        use test_bench::Bencher;

        use rand::{RngCore, SeedableRng};

        use super::{Hash, A, H};
        use crate::Tree;

        type Blake3Tree = Tree<Hash, H, A>;

        #[bench]
        fn blake3(b: &mut Bencher) {
            let tree = &mut Blake3Tree::new();
            let rng = &mut rand::rngs::StdRng::seed_from_u64(0xbeef);

            b.iter(|| {
                let pos = rng.next_u64();

                let mut hash_bytes = [0u8; 32];
                rng.fill_bytes(&mut hash_bytes);
                let hash = Hash(hash_bytes);

                tree.insert(pos, hash);
            });
        }
    }
}
