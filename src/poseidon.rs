// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::aggregate::Aggregate;
use crate::utils::init_array;

use dusk_plonk::prelude::*;
use dusk_poseidon::sponge::{gadget, hash};

#[derive(Clone, Copy)]
pub struct PoseidonItem<T> {
    pub scalar: BlsScalar,
    pub data: T,
}

impl<T, const H: usize, const A: usize> Aggregate<H, A> for PoseidonItem<T>
where
    T: Copy + Aggregate<H, A>,
{
    const EMPTY_SUBTREES: [Self; H] = init_array(|h| Self {
        scalar: BlsScalar::zero(),
        data: T::EMPTY_SUBTREES[h],
    });

    fn aggregate<'a, I>(items: I) -> Self
    where
        Self: 'a,
        I: Iterator<Item = &'a Self>,
    {
        todo!("do the logic")
    }
}
