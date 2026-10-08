// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::boxed::Box;
use core::array;
use core::cell::{Ref, RefCell};

use crate::{Aggregate, capacity};

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct Node<T, const H: usize, const A: usize> {
    item: RefCell<Option<T>>,
    pub(crate) children: [Option<Box<Node<T, H, A>>>; A],
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

    // `height` is this node's depth from the root: `0` for the root and `H`
    // for a leaf.
    pub(crate) fn item(&self, height: usize) -> Ref<'_, T> {
        if self.item.borrow().is_none() {
            // Compute our item, recursing into the children.
            let empty_subtree = &T::EMPTY_SUBTREE;
            let mut item_refs = [empty_subtree; A];

            let child_items: [Option<Ref<T>>; A] = array::from_fn(|i| {
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
    // item-less terminal node into an apparently populated leaf. `height` is
    // the node's depth from the root, from `0` at the root to `H` at a leaf.
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

#[cfg(feature = "rkyv-impl")]
mod rkyv_impl {
    use alloc::boxed::Box;
    use core::cell::RefCell;
    use core::{fmt, ptr};

    use bytecheck::{
        ArrayCheckError, CheckBytes, EnumCheckError, ErrorBox,
        StructCheckError, TupleStructCheckError,
    };
    use rkyv::ser::Serializer;
    use rkyv::validation::ArchiveContext;
    use rkyv::{
        Archive, Archived, Deserialize, Fallible, RelPtr, Resolver, Serialize,
        out_field,
    };

    use super::Node;

    pub struct ArchivedNode<T: Archive, const H: usize, const A: usize> {
        item: Archived<Option<T>>,
        children: Archived<[Option<Box<Node<T, H, A>>>; A]>,
    }

    // The tags of the `#[repr(u8)]` `ArchivedOption`, and its layout when it
    // holds `Some`, as rkyv writes it and the bytecheck derive reads it.
    const NONE: u8 = 0;
    const SOME: u8 = 1;

    #[repr(C)]
    struct ArchivedSome<T>(u8, T);

    #[derive(Debug)]
    struct HeightExceeded;

    impl fmt::Display for HeightExceeded {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an archived path exceeds the tree height")
        }
    }

    impl core::error::Error for HeightExceeded {}

    fn field_error(
        field_name: &'static str,
        error: impl bytecheck::Error,
    ) -> StructCheckError {
        StructCheckError {
            field_name,
            inner: ErrorBox::new(error),
        }
    }

    fn some_error(error: impl bytecheck::Error) -> EnumCheckError<u8> {
        EnumCheckError::InvalidTuple {
            variant_name: "Some",
            inner: TupleStructCheckError {
                field_index: 0,
                inner: ErrorBox::new(error),
            },
        }
    }

    // A derived check would recurse through `ArchivedOption` and `ArchivedBox`
    // without a depth limit. This one makes the same checks, with the same
    // error messages, but tracks the depth below the root and rejects children
    // of nodes at depth `H`, so it recurses at most `H` levels.
    impl<C, T, const H: usize, const A: usize> CheckBytes<C>
        for ArchivedNode<T, H, A>
    where
        C: ArchiveContext + ?Sized,
        C::Error: bytecheck::Error,
        T: Archive,
        Archived<Option<T>>: CheckBytes<C>,
    {
        type Error = StructCheckError;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            // SAFETY: The caller guarantees that `value` is aligned and points
            // to enough bytes for `Self`.
            unsafe { Self::check_at_depth(value, context, 0) }?;

            // SAFETY: The node and all nodes below it were validated above.
            let node = unsafe { &*value };
            let Self {
                item: _,
                children: _,
            } = node;
            Ok(node)
        }
    }

    impl<T: Archive, const H: usize, const A: usize> ArchivedNode<T, H, A> {
        // Checks a node `depth` levels below the root. `value` must be aligned
        // and point to enough bytes for `Self`.
        unsafe fn check_at_depth<C>(
            value: *const Self,
            context: &mut C,
            depth: usize,
        ) -> Result<(), StructCheckError>
        where
            C: ArchiveContext + ?Sized,
            C::Error: bytecheck::Error,
            Archived<Option<T>>: CheckBytes<C>,
        {
            // SAFETY: The caller guarantees that `value` is aligned and points
            // to enough bytes for `Self`, so each field pointer is aligned and
            // points to enough bytes for its field.
            unsafe {
                Archived::<Option<T>>::check_bytes(
                    ptr::addr_of!((*value).item),
                    context,
                )
            }
            .map_err(|error| field_error("item", error))?;

            for index in 0..A {
                // SAFETY: As above.
                let slot = unsafe { ptr::addr_of!((*value).children[index]) };
                let result = unsafe { Self::check_child(slot, context, depth) };
                result.map_err(|error| {
                    field_error("children", ArrayCheckError { index, error })
                })?;
            }

            Ok(())
        }

        // Checks a child slot of a node `depth` levels below the root like
        // `ArchivedOption` and `ArchivedBox` would, but recurses with the
        // child's depth. `slot` must be aligned and point to enough bytes for
        // the slot.
        unsafe fn check_child<C>(
            slot: *const Archived<Option<Box<Node<T, H, A>>>>,
            context: &mut C,
            depth: usize,
        ) -> Result<(), EnumCheckError<u8>>
        where
            C: ArchiveContext + ?Sized,
            C::Error: bytecheck::Error,
            Archived<Option<T>>: CheckBytes<C>,
        {
            // SAFETY: The tag of a `#[repr(u8)]` enum is its first byte.
            match unsafe { *slot.cast::<u8>() } {
                NONE => return Ok(()),
                SOME if depth < H => {}
                SOME => return Err(some_error(HeightExceeded)),
                tag => return Err(EnumCheckError::InvalidTag(tag)),
            }

            // SAFETY: A `Some` slot is laid out as `ArchivedSome`, holding an
            // `ArchivedBox`, which is a transparent `RelPtr`, in the archive.
            // Like `ArchivedBox::check_bytes`, this checks that the child lies
            // aligned and whole in the unclaimed subtree range, then claims the
            // child, so no other pointer can reach it or the nodes below it.
            unsafe {
                let slot = slot.cast::<ArchivedSome<RelPtr<Self>>>();
                let Ok(rel_ptr) = RelPtr::manual_check_bytes(
                    ptr::addr_of!((*slot).1),
                    context,
                );
                let child = context
                    .check_subtree_rel_ptr(rel_ptr)
                    .map_err(some_error)?;
                let range =
                    context.push_prefix_subtree(child).map_err(some_error)?;
                Self::check_at_depth(child, context, depth + 1)
                    .map_err(some_error)?;
                context.pop_prefix_range(range).map_err(some_error)
            }
        }
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

    // The serializer only writes valid tags and pointers, so these tests
    // corrupt a serialized archive to reach the checks of `check_child`.
    #[cfg(test)]
    mod tests {
        use alloc::string::{String, ToString};
        use core::{array, ptr};

        use rkyv::option::ArchivedOption;
        use rkyv::{AlignedVec, RelPtr};

        use super::ArchivedNode;
        use crate::Node;

        const H: usize = 2;
        const A: usize = 2;

        type TestNode = Node<(), H, A>;
        type TestArchivedNode = ArchivedNode<(), H, A>;

        // The positions in an archive of a child slot of the root, of the
        // relative pointer in it, and of the child it points to.
        struct Child {
            slot: usize,
            rel_ptr: usize,
            target: usize,
        }

        fn position<T>(bytes: &[u8], value: &T) -> usize {
            ptr::from_ref(value).addr() - bytes.as_ptr().addr()
        }

        // Archives a root with two populated children and checks that the
        // unmodified archive validates, so each test's change is the only
        // reason for its rejection.
        fn archive() -> (AlignedVec, [Child; A]) {
            let mut root = TestNode::new();
            root.insert(0, 0, ());
            root.insert(0, 2, ());
            let bytes = rkyv::to_bytes::<_, 128>(&root)
                .expect("Archiving a node should succeed");

            let archived = rkyv::check_archived_root::<TestNode>(&bytes)
                .expect("The unmodified archive should validate");
            let children = array::from_fn(|index| {
                let slot = &archived.children[index];
                let ArchivedOption::Some(child) = slot else {
                    panic!("The root should have child {index}");
                };
                // An `ArchivedBox` is a transparent `RelPtr`.
                Child {
                    slot: position(&bytes, slot),
                    rel_ptr: position(&bytes, child),
                    target: position(&bytes, child.get()),
                }
            });
            (bytes, children)
        }

        // Points the relative pointer at position `rel_ptr` to `target`.
        fn redirect(bytes: &mut AlignedVec, rel_ptr: usize, target: usize) {
            let out = bytes[rel_ptr..].as_mut_ptr().cast();
            // SAFETY: `out` points to the aligned relative pointer at
            // `rel_ptr` in `bytes`, and `emplace` only writes its offset.
            unsafe {
                RelPtr::<TestArchivedNode>::emplace(rel_ptr, target, out);
            }
        }

        fn rejection(bytes: &[u8]) -> String {
            rkyv::check_archived_root::<TestNode>(bytes)
                .map(drop)
                .expect_err("The modified archive must be rejected")
                .to_string()
        }

        #[test]
        fn child_with_invalid_tag_is_rejected() {
            let (mut bytes, [child, _]) = archive();
            // The tag of a `#[repr(u8)]` enum is its first byte.
            bytes[child.slot] = 2;

            let error = rejection(&bytes);
            assert!(error.contains("invalid tag for enum: 2"), "{error}");
        }

        #[test]
        fn child_pointer_past_the_buffer_is_rejected() {
            let (mut bytes, [child, _]) = archive();
            let past_end = bytes.len() + size_of::<TestArchivedNode>();
            redirect(&mut bytes, child.rel_ptr, past_end);

            let error = rejection(&bytes);
            assert!(error.contains("pointer out of bounds: base"), "{error}");
        }

        #[test]
        fn second_child_pointer_to_the_first_child_is_rejected() {
            let (mut bytes, [first, second]) = archive();
            // Give the second slot the first slot's offset, less the distance
            // between the slots, so that both point to the first child.
            redirect(&mut bytes, second.rel_ptr, first.target);

            let error = rejection(&bytes);
            assert!(error.contains("subtree pointer out of bounds"), "{error}");
        }
    }
}
