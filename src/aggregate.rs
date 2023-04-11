// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// A type that can be produced by aggregating multiple instances of itself, at
/// certain heights of the tree.
#[allow(clippy::module_name_repetitions)]
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
}
