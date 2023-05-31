// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::boxed::Box;
use core::cell::{Ref, RefCell};

#[cfg(feature = "rkyv-impl")]
use bytecheck::{CheckBytes, Error as BytecheckError};
#[cfg(feature = "rkyv-impl")]
use rkyv::{
    ser::Serializer, validation::ArchiveContext, Archive, Deserialize,
    Fallible, Serialize,
};

use crate::{capacity, init_array, Aggregate};

#[derive(Debug, Clone, PartialEq, Eq)]
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
    item: RefCell<Option<T>>,
    #[cfg_attr(feature = "rkyv-impl", omit_bounds, archive_attr(omit_bounds))]
    pub(crate) children: [Option<Box<Node<T, H, A>>>; A],
}

impl<T, const H: usize, const A: usize> Node<T, H, A>
where
    T: Aggregate<H, A>,
{
    const INIT_NODE: Option<Box<Node<T, H, A>>> = None;

    pub(crate) const fn new() -> Self {
        debug_assert!(H > 0, "Height must be larger than zero");
        debug_assert!(A > 0, "Arity must be larger than zero");

        Self {
            item: RefCell::new(None),
            children: [Self::INIT_NODE; A],
        }
    }

    pub(crate) fn item(&self, height: usize) -> Ref<T> {
        if self.item.borrow().is_none() {
            let empty = &T::EMPTY_SUBTREES[height + 1];
            let mut item_refs = [empty; A];

            let child_items: [Option<Ref<T>>; A] = init_array(|i| {
                self.children[i].as_ref().map(|item| item.item(height))
            });

            let mut has_children = false;
            item_refs.iter_mut().zip(&child_items).for_each(|(r, c)| {
                if let Some(c) = c {
                    *r = c;
                    has_children = true;
                }
            });

            if has_children {
                self.item.replace(Some(T::aggregate(item_refs)));
            } else {
                self.item.replace(Some(T::EMPTY_SUBTREES[height]));
            }
        }

        // unwrapping is ok since we ensure it exists
        Ref::map(self.item.borrow(), |item| item.as_ref().unwrap())
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
            self.item.replace(Some(item.into()));
            return;
        }
        self.item.replace(None);

        let (child_index, child_pos) = Self::child_location(height, position);

        let child = &mut self.children[child_index];
        if child.is_none() {
            *child = Some(Box::new(Node::new()));
        }

        // We just inserted a child at the given index.
        let child = self.children[child_index].as_mut().unwrap();
        Self::insert(child, height + 1, child_pos, item);
    }

    /// Returns the removed element, together with if there are any siblings
    /// left in the branch.
    ///
    /// # Panics
    /// If an element does not exist at the given position.
    pub(crate) fn remove(&mut self, height: usize, position: u64) -> (T, bool) {
        if height == H {
            // unwrapping is ok since leaves are always filled
            let item = self.item.take().unwrap();
            return (item, false);
        }
        self.item.replace(None);

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

        (removed_item, has_children)
    }
}
