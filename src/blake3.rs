// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Aggregate;
use blake3::{Hash as Blake3Hash, Hasher};

const EMPTY_HASH: Item = Item([0; 32]);

#[derive(Debug, Clone, Copy)]
pub struct Item([u8; 32]);

impl From<Blake3Hash> for Item {
    fn from(h: Blake3Hash) -> Self {
        Self(h.into())
    }
}

impl<const H: usize, const A: usize> Aggregate<H, A> for Item {
    const EMPTY_SUBTREES: [Self; H] = [EMPTY_HASH; H];

    fn aggregate(items: [&Self; A]) -> Self {
        let mut hasher = Hasher::new();
        for item in items {
            hasher.update(&item.0);
        }
        hasher.finalize().into()
    }
}

impl Item {
    pub fn new(bytes: [u8; 32]) -> Self {
        Item(bytes)
    }
}
