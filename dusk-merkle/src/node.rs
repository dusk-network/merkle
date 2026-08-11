// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::boxed::Box;
use core::cell::{Ref, RefCell};

use crate::{Aggregate, capacity, init_array};

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct Node<T, const H: usize, const A: usize> {
    item: RefCell<Option<T>>,
    pub(crate) children: [Option<Box<Node<T, H, A>>>; A],
}

#[cfg(feature = "rkyv-impl")]
impl<T, const H: usize, const A: usize> Node<T, H, A> {
    pub(crate) fn clear_internal_caches(&self, height: usize) {
        if height == H {
            return;
        }

        self.item.replace(None);
        for child in self.children.iter().flatten() {
            child.clear_internal_caches(height + 1);
        }
    }
}

impl<T, const H: usize, const A: usize> Node<T, H, A>
where
    T: Aggregate<A>,
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

    pub(crate) fn item(&self, height: usize) -> Ref<'_, T> {
        if self.item.borrow().is_none() {
            // Compute our item, recursing into the children.
            let empty_subtree = &T::EMPTY_SUBTREE;
            let mut item_refs = [empty_subtree; A];

            let child_items: [Option<Ref<T>>; A] = init_array(|i| {
                self.children[i]
                    .as_ref()
                    .and_then(|item| item.populated_item(height + 1))
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
                self.item.replace(Some(T::EMPTY_SUBTREE));
            }
        }

        // Unwrapping is safe because we ensure the item exists above.
        Ref::map(self.item.borrow(), |item| item.as_ref().unwrap())
    }

    // Unlike `item`, this does not allow lazy cache population to turn an
    // item-less terminal node into an apparently populated leaf.
    pub(crate) fn populated_item(&self, height: usize) -> Option<Ref<'_, T>> {
        if height == H && !self.has_cached_item() {
            return None;
        }
        Some(self.item(height))
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

    // Cache presence is not a durable inserted-leaf invariant for malformed
    // archives: `item()` can lazily populate an item-less leaf.
    pub(crate) fn has_cached_item(&self) -> bool {
        self.item.borrow().is_some()
    }

    #[cfg(all(test, feature = "rkyv-impl"))]
    pub(crate) fn replace_cached_item(&self, item: Option<T>) {
        self.item.replace(item);
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

    /// Returns the removed element, together with whether there are any
    /// siblings left in the branch. Returns `None` if the path is incomplete.
    pub(crate) fn remove(
        &mut self,
        height: usize,
        position: u64,
    ) -> Option<(T, bool)> {
        if height == H {
            // This is fallible for a cold item-less leaf, but a lazily warmed
            // cache cannot establish whether an archived leaf was inserted.
            return self.item.take().map(|item| (item, false));
        }

        let (child_index, child_pos) = Self::child_location(height, position);
        let child = self.children.get_mut(child_index)?.as_mut()?;
        let (removed_item, child_has_children) =
            Self::remove(child, height + 1, child_pos)?;

        self.item.replace(None);
        if !child_has_children {
            self.children[child_index] = None;
        }

        let has_children = self.children.iter().any(Option::is_some);
        Some((removed_item, has_children))
    }
}

// Allow `unsafe_op_in_unsafe_fn` because the `CheckBytes` derive macro from
// bytecheck 0.6 generates unsafe operations without `unsafe {}` blocks, which
// is not edition-2024-compliant.
#[cfg(feature = "rkyv-impl")]
#[allow(unsafe_op_in_unsafe_fn)]
mod rkyv_impl {
    use alloc::boxed::Box;
    use core::cell::RefCell;

    use bytecheck::CheckBytes;
    use rkyv::ser::Serializer;
    use rkyv::{
        Archive, Archived, Deserialize, Fallible, Resolver, Serialize,
        out_field,
    };

    use super::Node;

    #[derive(CheckBytes)]
    #[check_bytes(
        bound = "__C: rkyv::validation::ArchiveContext, <__C as rkyv::Fallible>::Error: bytecheck::Error"
    )]
    pub struct ArchivedNode<T: Archive, const H: usize, const A: usize> {
        item: Archived<Option<T>>,
        #[omit_bounds]
        children: Archived<[Option<Box<Node<T, H, A>>>; A]>,
    }

    pub struct NodeResolver<T: Archive, const H: usize, const A: usize> {
        item: Resolver<Option<T>>,
        children: Resolver<[Option<Box<Node<T, H, A>>>; A]>,
    }

    impl<T: Archive, const H: usize, const A: usize> ArchivedNode<T, H, A> {
        pub(crate) fn has_item(&self) -> bool {
            self.item.is_some()
        }

        pub(crate) fn child(&self, index: usize) -> Option<&Self> {
            self.children.get(index)?.as_deref()
        }

        pub(crate) fn has_children(&self) -> bool {
            self.children
                .iter()
                .any(rkyv::option::ArchivedOption::is_some)
        }
    }

    impl<T, const H: usize, const A: usize> Archive for Node<T, H, A>
    where
        T: Archive,
    {
        type Archived = ArchivedNode<T, H, A>;
        type Resolver = NodeResolver<T, H, A>;

        unsafe fn resolve(
            &self,
            pos: usize,
            resolver: Self::Resolver,
            out: *mut Self::Archived,
        ) {
            let (item_pos, item) = out_field!(out.item);
            let (children_pos, children) = out_field!(out.children);

            // SAFETY: `out` points to a valid `ArchivedNode` allocation, and
            // the field pointers and offsets produced by `out_field!` are
            // correct. The caller guarantees that `pos` matches the position
            // of `out` in the output buffer.
            unsafe {
                self.item
                    .borrow()
                    .resolve(pos + item_pos, resolver.item, item);
                self.children.resolve(
                    pos + children_pos,
                    resolver.children,
                    children,
                );
            }
        }
    }

    impl<S, T, const H: usize, const A: usize> Serialize<S> for Node<T, H, A>
    where
        S: Serializer + ?Sized,
        T: Archive + Serialize<S>,
    {
        fn serialize(
            &self,
            serializer: &mut S,
        ) -> Result<Self::Resolver, S::Error> {
            let item = self.item.borrow();

            let item = item.serialize(serializer)?;
            let children = self.children.serialize(serializer)?;

            Ok(Self::Resolver { item, children })
        }
    }

    impl<D, T, const H: usize, const A: usize> Deserialize<Node<T, H, A>, D>
        for ArchivedNode<T, H, A>
    where
        D: Fallible + ?Sized,
        T: Archive,
        Archived<T>: Deserialize<T, D>,
    {
        fn deserialize(
            &self,
            deserializer: &mut D,
        ) -> Result<Node<T, H, A>, D::Error> {
            let item = self.item.deserialize(deserializer)?;
            let children = self.children.deserialize(deserializer)?;
            Ok(Node {
                item: RefCell::new(item),
                children,
            })
        }
    }
}
