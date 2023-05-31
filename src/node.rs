// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::boxed::Box;
use core::mem;

#[cfg(feature = "rkyv-impl")]
use bytecheck::{CheckBytes, Error as BytecheckError};
#[cfg(feature = "rkyv-impl")]
use rkyv::{
    ser::Serializer, validation::ArchiveContext, Archive, Deserialize,
    Fallible, Serialize,
};

use crate::{capacity, Aggregate};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "rkyv-impl",
    derive(Archive, Serialize, Deserialize),
    archive(bound(serialize = "__S: Serializer")),
    archive_attr(
        derive(CheckBytes),
        doc(hidden),
        check_bytes(
            bound = "__C: ArchiveContext, <__C as Fallible>::Error: BytecheckError"
        )
    )
)]
#[doc(hidden)]
pub struct Node<T, const H: usize, const A: usize> {
    pub(crate) item: T,
    #[cfg_attr(feature = "rkyv-impl", omit_bounds, archive_attr(omit_bounds))]
    pub(crate) children: [Option<Box<Node<T, H, A>>>; A],
}

impl<T, const H: usize, const A: usize> Node<T, H, A>
where
    T: Aggregate<H, A>,
{
    const INIT_NODE: Option<Box<Node<T, H, A>>> = None;

    pub(crate) const fn new(item: T) -> Self {
        debug_assert!(H > 0, "Height must be larger than zero");
        debug_assert!(A > 0, "Arity must be larger than zero");

        Self {
            item,
            children: [Self::INIT_NODE; A],
        }
    }

    fn compute_item(&mut self, height: usize) {
        let empty = &T::EMPTY_SUBTREES[height];

        self.item = T::aggregate(
            self.children
                .iter()
                .map(|node| node.as_ref().map(|node| &node.as_ref().item))
                .map(|item| item.unwrap_or(empty)),
        );
    }

    pub(crate) fn child_location(height: usize, position: u64) -> (usize, u64) {
        let child_cap = capacity(A as u64, H - height - 1);

        // Casting to a `usize` should be fine, since the index should be within
        // the `[0, A[` bound anyway.
        #[allow(clippy::cast_possible_truncation)]
        let child_index = (position / child_cap) as usize;
        let child_pos = position % child_cap;

        (child_index, child_pos)
    }

    pub(crate) fn insert(
        &mut self,
        height: usize,
        position: u64,
        item: impl Into<T>,
    ) {
        if height == H {
            self.item = item.into();
            return;
        }

        let (child_index, child_pos) = Self::child_location(height, position);

        let child = &mut self.children[child_index];
        if child.is_none() {
            *child = Some(Box::new(Node::new(T::EMPTY_SUBTREES[height])));
        }

        // We just inserted a child at the given index.
        let child = self.children[child_index].as_mut().unwrap();
        Self::insert(child, height + 1, child_pos, item);

        self.compute_item(height);
    }

    /// Returns the removed element, together with if there are any siblings
    /// left in the branch.
    ///
    /// # Panics
    /// If an element does not exist at the given position.
    pub(crate) fn remove(&mut self, height: usize, position: u64) -> (T, bool) {
        if height == H {
            let mut item = T::EMPTY_SUBTREES[0];
            mem::swap(&mut self.item, &mut item);
            return (item, false);
        }

        let (child_index, child_pos) = Self::child_location(height, position);

        let child = self.children[child_index]
            .as_mut()
            .expect("There should be a child at this position");
        let (removed_item, child_has_children) =
            Self::remove(child, height + 1, child_pos);

        if !child_has_children {
            self.children[child_index] = None;
        }

        let mut has_children = false;
        for child in &self.children {
            if child.is_some() {
                has_children = true;
                break;
            }
        }

        if has_children {
            self.compute_item(height);
        }

        (removed_item, has_children)
    }
}
