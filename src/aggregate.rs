// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// A type that can be produced by aggregating multiple instances of itself, at
/// certain heights of the tree.
pub trait Aggregate {
    /// Aggregate `items` to produce a single one at the given `height`.
    fn aggregate<'a, I>(height: usize, items: I) -> Self
    where
        Self: 'a,
        I: ExactSizeIterator<Item = Option<&'a Self>>;
}

#[cfg(feature = "blake3")]
mod blake {
    use super::Aggregate;
    use blake3::{Hash, Hasher};

    impl Aggregate for Hash {
        fn aggregate<'a, I>(_: usize, items: I) -> Self
        where
            Self: 'a,
            I: ExactSizeIterator<Item = Option<&'a Self>>,
        {
            let mut hasher = Hasher::new();
            for item in items {
                match item {
                    Some(item) => hasher.update(item.as_bytes()),
                    None => hasher.update(&[0u8; 32]),
                };
            }
            hasher.finalize()
        }
    }

    #[cfg(test)]
    #[cfg(feature = "bench")]
    mod bench {
        use test::Bencher;

        use blake3::Hash;
        use rand::{RngCore, SeedableRng};

        use crate::Tree;

        const H: usize = 32;
        const A: usize = 4;

        type Blake3Tree = Tree<Hash, H, A>;

        #[bench]
        fn blake3(b: &mut Bencher) {
            let tree = &mut Blake3Tree::new();
            let rng = &mut rand::rngs::StdRng::seed_from_u64(0xbeef);

            b.iter(|| {
                let pos = rng.next_u64();

                let mut hash_bytes = [0u8; 32];
                rng.fill_bytes(&mut hash_bytes);
                let hash = Hash::from(hash_bytes);

                tree.insert(pos, hash);
            });
        }
    }
}
