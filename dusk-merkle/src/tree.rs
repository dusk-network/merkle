// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::BTreeSet;
use core::cell::Ref;

use crate::{Aggregate, Node, Opening, Walk, capacity};

/// A sparse Merkle tree.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv-impl", derive(rkyv::Archive, rkyv::Serialize))]
pub struct Tree<T, const H: usize, const A: usize> {
    pub(crate) root: Node<T, H, A>,
    positions: BTreeSet<u64>,
}

impl<T, const H: usize, const A: usize> Default for Tree<T, H, A>
where
    T: Aggregate<A>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const H: usize, const A: usize> Tree<T, H, A>
where
    T: Aggregate<A>,
{
    /// Create a new merkle tree with the given initial `root`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: Node::new(),
            positions: BTreeSet::new(),
        }
    }

    /// Insert an `item` at the given `position` in the tree.
    ///
    /// # Panics
    /// If `index >= capacity`.
    pub fn insert(&mut self, index: u64, item: impl Into<T>) {
        let capacity = self.capacity();

        assert!(
            index < capacity,
            "index out of bounds: \
             the capacity is {capacity} but the index is {index}"
        );

        self.root.insert(0, index, item);
        self.positions.insert(index);
    }

    /// Remove and return the item at the given `position` in the tree if it
    /// exists.
    pub fn remove(&mut self, position: u64) -> Option<T> {
        if !self.has_recorded_position(position) {
            return None;
        }

        let (item, _) = self.root.remove(0, position)?;
        self.positions.remove(&position);

        Some(item)
    }

    /// Returns the [`Opening`] for the given `position` if it exists.
    pub fn opening(&self, position: u64) -> Option<Opening<T, H, A>>
    where
        T: Clone,
    {
        if !self.has_recorded_position(position) {
            return None;
        }
        Opening::new(self, position)
    }

    // Capacity must be checked before private path traversal: child indices
    // are derived through a potentially truncating `u64` to `usize` cast.
    fn has_recorded_position(&self, position: u64) -> bool {
        position < self.capacity() && self.positions.contains(&position)
    }

    /// Returns a [`Walk`] through the tree, proceeding according to the
    /// `walker` function.
    ///
    /// A walk starts from the root of the tree, and "drills down" according to
    /// the output of the walker function. The function should return `true` or
    /// `false`, indicating whether the iterator should continue along the
    /// tree's path.
    pub fn walk<W>(&self, walker: W) -> Walk<'_, T, W, H, A>
    where
        W: Fn(&T) -> bool,
    {
        Walk::new(self, walker)
    }

    /// Get the root of the merkle tree.
    pub fn root(&self) -> Ref<'_, T> {
        self.root.item(0)
    }

    /// Returns the root of the smallest sub-tree that holds all the leaves.
    pub fn smallest_subtree(&self) -> (Ref<'_, T>, usize) {
        let mut smallest_node = &self.root;
        let mut height = H;
        loop {
            let mut children = smallest_node.children.iter().flatten();
            match children.next() {
                // when the root has no children, the tree is empty and we
                // return its root. This is only possible because the empty
                // subtrees are the same for each level.
                None => return (self.root(), 0),
                Some(child) => {
                    // if there is no more than one child and we are not at the
                    // end of the tree, we need to continue to traverse
                    if children.next().is_none() && height > 1 {
                        smallest_node = child;
                    }
                    // otherwise we return the item of the current node and the
                    // current height as the root and height of the smallest
                    // subtree
                    else {
                        return (smallest_node.item(H - height), height);
                    }
                }
            }
            height -= 1;
        }
    }

    /// Returns true if the tree contains a leaf at the given `position`.
    pub fn contains(&self, position: u64) -> bool {
        self.positions.contains(&position)
    }

    /// Returns the number of elements that have been inserted into the tree.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.positions.len() as u64
    }

    /// Returns `true` if the tree is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The maximum number of leaves in the tree, i.e. its capacity.
    #[must_use]
    pub const fn capacity(&self) -> u64 {
        capacity(A as u64, H)
    }
}

// Extend rkyv's structural byte checks with tree invariants without changing
// the archived representation.
#[cfg(feature = "rkyv-impl")]
mod rkyv_impl {
    use alloc::boxed::Box;
    use alloc::collections::BTreeSet;
    use core::{fmt, ptr};

    use bytecheck::{CheckBytes, Error, StructCheckError};
    use rkyv::{Archive, Archived, Deserialize, Fallible};

    use super::{ArchivedTree, Tree};
    use crate::Node;

    #[derive(Debug)]
    struct ArchiveInvariantError(&'static str);

    impl fmt::Display for ArchiveInvariantError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl core::error::Error for ArchiveInvariantError {}

    fn field_error(
        field_name: &'static str,
        error: impl Error,
    ) -> StructCheckError {
        StructCheckError {
            field_name,
            inner: Box::new(error),
        }
    }

    fn invariant_error(message: &'static str) -> StructCheckError {
        field_error("positions", ArchiveInvariantError(message))
    }

    fn checked_capacity<const H: usize, const A: usize>() -> Option<u64> {
        if H == 0 || A == 0 {
            return None;
        }

        let arity = u64::try_from(A).ok()?;
        let height = u32::try_from(H).ok()?;
        arity.checked_pow(height)
    }

    fn validate_node<T, I, const H: usize, const A: usize>(
        node: &Archived<Node<T, H, A>>,
        height: usize,
        position: u64,
        capacity: u64,
        positions: &mut I,
    ) -> Result<(), ArchiveInvariantError>
    where
        T: Archive,
        I: Iterator<Item = u64>,
    {
        if height == H {
            if node.has_children() {
                return Err(ArchiveInvariantError(
                    "an archived node extends below the tree height",
                ));
            }
            if !node.has_item() {
                return Err(ArchiveInvariantError(
                    "an archived leaf has no item",
                ));
            }
            return match positions.next() {
                Some(expected) if expected == position => Ok(()),
                _ => Err(ArchiveInvariantError(
                    "archived leaves and recorded positions do not match",
                )),
            };
        }

        let child_capacity = capacity
            / u64::try_from(A).map_err(|_| {
                ArchiveInvariantError("the archived tree arity exceeds u64")
            })?;
        let mut has_children = false;
        for index in 0..A {
            if let Some(child) = node.child(index) {
                has_children = true;
                let offset = u64::try_from(index)
                    .ok()
                    .and_then(|index| index.checked_mul(child_capacity))
                    .and_then(|offset| position.checked_add(offset))
                    .ok_or(ArchiveInvariantError(
                        "an archived path exceeds the tree capacity",
                    ))?;
                validate_node(
                    child,
                    height + 1,
                    offset,
                    child_capacity,
                    positions,
                )?;
            }
        }

        if !has_children && height != 0 {
            return Err(ArchiveInvariantError(
                "an archived path has no populated leaf",
            ));
        }

        Ok(())
    }

    impl<D, T, const H: usize, const A: usize> Deserialize<Tree<T, H, A>, D>
        for ArchivedTree<T, H, A>
    where
        D: Fallible + ?Sized,
        T: Archive,
        Archived<Node<T, H, A>>: Deserialize<Node<T, H, A>, D>,
        Archived<BTreeSet<u64>>: Deserialize<BTreeSet<u64>, D>,
    {
        fn deserialize(
            &self,
            deserializer: &mut D,
        ) -> Result<Tree<T, H, A>, D::Error> {
            let root = self.root.deserialize(deserializer)?;
            root.clear_internal_caches(0);
            let positions = self.positions.deserialize(deserializer)?;
            Ok(Tree { root, positions })
        }
    }

    fn validate_archive<T, const H: usize, const A: usize>(
        tree: &ArchivedTree<T, H, A>,
    ) -> Result<(), StructCheckError>
    where
        T: Archive,
    {
        let capacity = checked_capacity::<H, A>().ok_or_else(|| {
            invariant_error(
                "tree height and arity must be nonzero and fit in u64 capacity",
            )
        })?;

        if tree.positions.iter().any(|position| *position >= capacity) {
            return Err(invariant_error(
                "an archived position is outside the tree capacity",
            ));
        }

        let mut positions = tree.positions.iter().copied();
        validate_node(&tree.root, 0, 0, capacity, &mut positions)
            .map_err(|error| field_error("root", error))?;
        if positions.next().is_some() {
            return Err(invariant_error(
                "archived leaves and recorded positions do not match",
            ));
        }

        Ok(())
    }

    impl<C, T, const H: usize, const A: usize> CheckBytes<C>
        for ArchivedTree<T, H, A>
    where
        C: ?Sized,
        T: Archive,
        Archived<Node<T, H, A>>: CheckBytes<C>,
        Archived<BTreeSet<u64>>: CheckBytes<C>,
    {
        // Keep the derive-generated error type for API compatibility while
        // extending the field checks with semantic tree validation.
        type Error = StructCheckError;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            // SAFETY: The caller guarantees that `value` is aligned and points
            // to enough bytes for `Self`; each field validator checks its own
            // archived representation and referenced data.
            unsafe {
                <Archived<Node<T, H, A>> as CheckBytes<C>>::check_bytes(
                    ptr::addr_of!((*value).root),
                    context,
                )
            }
            .map_err(|error| field_error("root", error))?;

            // SAFETY: This is the second field of the same caller-validated
            // `ArchivedTree` allocation. Its validator checks all B-tree data
            // before semantic validation reads it.
            unsafe {
                <Archived<BTreeSet<u64>> as CheckBytes<C>>::check_bytes(
                    ptr::addr_of!((*value).positions),
                    context,
                )
            }
            .map_err(|error| field_error("positions", error))?;

            // SAFETY: Both fields and all referenced archived data have been
            // structurally validated above.
            let tree = unsafe { &*value };
            validate_archive(tree)?;
            Ok(tree)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Aggregate<A> for u8 {
        const EMPTY_SUBTREE: Self = 0;

        fn aggregate(items: [&Self; A]) -> Self {
            items.into_iter().sum()
        }
    }

    const H: usize = 3;
    const A: usize = 2;

    type SumTree = Tree<u8, H, A>;

    #[test]
    fn tree_insertion() {
        let mut tree = SumTree::new();

        tree.insert(5, 42);
        tree.insert(6, 42);
        tree.insert(5, 42);

        assert_eq!(
            tree.len(),
            2,
            "Three items were inserted, but one was in the same position as another"
        );
    }

    #[test]
    fn tree_deletion() {
        let mut tree = SumTree::new();

        tree.insert(5, 42);
        tree.insert(6, 42);
        tree.insert(5, 42);

        tree.remove(5);
        tree.remove(4);

        assert_eq!(
            tree.len(),
            1,
            "There should be one element left in the tree"
        );

        assert_eq!(*tree.root(), 42);

        tree.remove(6);
        assert!(tree.is_empty(), "The tree should be empty");
        assert_eq!(
            *tree.root(),
            u8::EMPTY_SUBTREE,
            "Since the tree is empty the root should be the first empty item"
        );
    }

    #[test]
    #[should_panic(
        expected = "index out of bounds: the capacity is 8 but the index is 8"
    )]
    fn tree_insertion_out_of_bounds() {
        let mut tree = SumTree::new();
        tree.insert(tree.capacity(), 42);
    }

    #[test]
    fn inconsistent_tree_path_returns_none() {
        let mut tree = SumTree::new();
        let position = 5;
        tree.insert(position, 42);

        let (child_index, _) = Node::<u8, H, A>::child_location(0, position);
        tree.root.children[child_index] = None;

        let len = tree.len();
        let root = *tree.root();

        assert!(tree.opening(position).is_none());
        assert!(tree.remove(position).is_none());
        assert!(tree.contains(position));
        assert_eq!(tree.len(), len);
        assert_eq!(*tree.root(), root);
    }

    #[test]
    fn inconsistent_tree_path_below_root_returns_none() {
        let mut tree = SumTree::new();
        let position = 5;
        tree.insert(position, 42);

        let (child_index, child_position) =
            Node::<u8, H, A>::child_location(0, position);
        let child = tree.root.children[child_index]
            .as_mut()
            .expect("The inserted item must populate the root child");
        let (child_index, _) =
            Node::<u8, H, A>::child_location(1, child_position);
        child.children[child_index] = None;

        let len = tree.len();
        let root = *tree.root();

        assert!(tree.opening(position).is_none());
        assert!(tree.remove(position).is_none());
        assert!(tree.contains(position));
        assert_eq!(tree.len(), len);
        assert_eq!(*tree.root(), root);
    }

    #[test]
    fn out_of_bounds_tree_position_returns_none() {
        let mut tree = SumTree::new();
        tree.insert(0, 42);

        let position = 1 << 34;
        tree.positions.insert(position);

        let len = tree.len();
        let root = *tree.root();

        assert!(!tree.has_recorded_position(position));
        assert!(tree.opening(position).is_none());
        assert!(tree.remove(position).is_none());
        assert!(tree.contains(position));
        assert_eq!(tree.len(), len);
        assert_eq!(*tree.root(), root);
    }

    // create test tree for shrunken root:

    type RangeTree = Tree<Option<Range>, H, A>;

    // min and max are either
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Range {
        min: u64,
        max: u64,
    }

    impl Range {
        pub fn new(min: u64, max: u64) -> Self {
            Range { min, max }
        }
    }

    impl Aggregate<A> for Option<Range> {
        const EMPTY_SUBTREE: Self = None;

        fn aggregate(items: [&Self; A]) -> Self {
            let mut bh_range = None;

            for item in items {
                bh_range = match (bh_range, item.as_ref()) {
                    (None, None) => None,
                    (None, Some(r)) => Some(*r),
                    (Some(r), None) => Some(r),
                    (Some(bh_range), Some(item_bh_range)) => {
                        let min =
                            core::cmp::min(item_bh_range.min, bh_range.min);
                        let max =
                            core::cmp::max(item_bh_range.max, bh_range.max);
                        Some(Range { min, max })
                    }
                };
            }

            bh_range
        }
    }

    #[test]
    fn smallest_subtree() {
        let empty_root: Option<Range> = None;

        let mut tree = RangeTree::new();
        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, empty_root);
        assert_eq!(height, 0);
        drop(smallest_subtree);

        tree.insert(0, Some(Range::new(0, 0)));

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, Some(Range::new(0, 0)));
        assert_eq!(height, 1);
        drop(smallest_subtree);

        tree.insert(1, Some(Range::new(1, 1)));

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, Some(Range::new(0, 1)));
        assert_eq!(height, 1);
        drop(smallest_subtree);

        tree.insert(2, Some(Range::new(2, 2)));

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, Some(Range::new(0, 2)));
        assert_eq!(height, 2);
        drop(smallest_subtree);

        tree.insert(3, Some(Range::new(3, 3)));

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, Some(Range::new(0, 3)));
        assert_eq!(height, 2);
        drop(smallest_subtree);

        tree.insert(7, Some(Range::new(7, 7)));

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, Some(Range::new(0, 7)));
        assert_eq!(height, 3);
        drop(smallest_subtree);

        tree.remove(0);
        tree.remove(1);
        tree.remove(2);

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, Some(Range::new(3, 7)));
        assert_eq!(height, 3);
        drop(smallest_subtree);

        tree.remove(3);
        tree.insert(4, Some(Range::new(4, 4)));

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, Some(Range::new(4, 7)));
        assert_eq!(height, 2);
        drop(smallest_subtree);

        tree.remove(4);

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert_eq!(*smallest_subtree, Some(Range::new(7, 7)));
        assert_eq!(height, 1);
        drop(smallest_subtree);

        tree.remove(7);

        let (smallest_subtree, height) = tree.smallest_subtree();
        assert!(smallest_subtree.is_none());
        assert_eq!(height, 0);
    }

    #[cfg(feature = "rkyv-impl")]
    mod rkyv_impl {
        extern crate std;

        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use std::panic::{AssertUnwindSafe, catch_unwind};

        use super::{A, H, SumTree};
        use crate::{Aggregate, Node};

        const POSITION: u64 = 5;

        fn archive(tree: &SumTree) -> Vec<u8> {
            rkyv::to_bytes::<_, 128>(tree)
                .expect("Archiving a tree should succeed")
                .to_vec()
        }

        fn assert_rejected_without_unwind(case: &str, tree: &SumTree) {
            let tree_bytes = archive(tree);
            let result = catch_unwind(AssertUnwindSafe(|| {
                rkyv::from_bytes::<SumTree>(&tree_bytes)
            }));

            match result {
                Ok(Err(_)) => {}
                Ok(Ok(_)) => panic!("{case}: malformed archive was accepted"),
                Err(_) => panic!("{case}: archive rejection unwound"),
            }
        }

        fn populated_tree() -> SumTree {
            let mut tree = SumTree::new();
            tree.insert(POSITION, 42);
            tree
        }

        fn decode_without_unwind(case: &str, tree: &SumTree) -> SumTree {
            let tree_bytes = archive(tree);
            let result = catch_unwind(AssertUnwindSafe(|| {
                rkyv::from_bytes::<SumTree>(&tree_bytes)
            }));

            match result {
                Ok(Ok(tree)) => tree,
                Ok(Err(error)) => {
                    panic!(
                        "{case}: structurally valid archive was rejected: {error}"
                    )
                }
                Err(_) => panic!("{case}: checked deserialization unwound"),
            }
        }

        fn node_at_height_mut(
            tree: &mut SumTree,
            target_height: usize,
            position: u64,
        ) -> &mut Node<u8, H, A> {
            let mut node = &mut tree.root;
            let mut child_position = position;
            for height in 0..target_height {
                let (child_index, next_position) =
                    Node::<u8, H, A>::child_location(height, child_position);
                node = node.children[child_index]
                    .as_mut()
                    .expect("The inserted items must populate the path");
                child_position = next_position;
            }
            node
        }

        fn replace_leaf_with_itemless_node(
            node: &mut Node<u8, H, A>,
            height: usize,
            position: u64,
        ) {
            let (child_index, child_position) =
                Node::<u8, H, A>::child_location(height, position);
            if height + 1 == H {
                node.children[child_index] = Some(Box::new(Node::new()));
                return;
            }

            let child = node.children[child_index]
                .as_mut()
                .expect("The inserted item must populate every path node");
            replace_leaf_with_itemless_node(child, height + 1, child_position);
        }

        fn tree_with_itemless_leaf() -> SumTree {
            let mut tree = populated_tree();
            replace_leaf_with_itemless_node(&mut tree.root, 0, POSITION);
            tree
        }

        #[test]
        fn valid_tree_round_trip_and_operations() {
            let tree = populated_tree();
            let mut decoded = rkyv::from_bytes::<SumTree>(&archive(&tree))
                .expect("Deserializing a valid tree should succeed");

            assert_eq!(tree, decoded);
            assert!(
                decoded
                    .opening(POSITION)
                    .is_some_and(|opening| opening.verify(42))
            );
            assert_eq!(decoded.remove(POSITION), Some(42));

            let round_trip = rkyv::from_bytes::<SumTree>(&archive(&decoded))
                .expect("A valid modified tree should still round-trip");
            assert_eq!(decoded, round_trip);
        }

        fn valid_two_leaf_tree() -> SumTree {
            let mut tree = SumTree::new();
            tree.insert(4, 20);
            tree.insert(5, 22);
            tree
        }

        type OperationResults =
            (u8, Option<crate::Opening<u8, H, A>>, (u8, usize), Vec<u8>);

        fn operation_results_without_unwind(
            case: &str,
            tree: &SumTree,
            position: u64,
        ) -> OperationResults {
            catch_unwind(AssertUnwindSafe(|| {
                let root = *tree.root();
                let opening = tree.opening(position);
                let (smallest, height) = tree.smallest_subtree();
                let smallest = (*smallest, height);
                let walk = tree
                    .walk(|item| *item <= 42)
                    .map(|item| *item)
                    .collect::<Vec<_>>();
                (root, opening, smallest, walk)
            }))
            .unwrap_or_else(|_| {
                panic!("{case}: decoded tree operation unwound")
            })
        }

        fn assert_decodes_like(
            case: &str,
            archived: &SumTree,
            expected: &SumTree,
            position: u64,
        ) {
            let expected = operation_results_without_unwind(
                "canonical tree",
                expected,
                position,
            );
            let decoded = decode_without_unwind(case, archived);
            assert!(
                !decoded.root.has_cached_item(),
                "{case}: the decoded root cache must be cold"
            );
            assert_eq!(
                operation_results_without_unwind(case, &decoded, position),
                expected,
                "{case}: decoded operations must use aggregates derived from leaves"
            );
        }

        #[test]
        fn noncanonical_empty_root_cache_is_discarded() {
            let expected = SumTree::new();
            let archived = SumTree::new();
            archived.root.replace_cached_item(Some(99));

            assert_decodes_like("empty root cache", &archived, &expected, 0);
        }

        #[test]
        fn noncanonical_populated_root_cache_is_discarded() {
            let expected = populated_tree();
            let archived = expected.clone();
            archived.root.replace_cached_item(Some(99));

            assert_decodes_like(
                "populated root cache",
                &archived,
                &expected,
                POSITION,
            );
        }

        #[test]
        fn noncanonical_intermediate_cache_is_discarded() {
            let expected = valid_two_leaf_tree();
            let mut archived = expected.clone();
            node_at_height_mut(&mut archived, H - 1, 4)
                .replace_cached_item(Some(99));
            archived.root.replace_cached_item(None);

            assert_decodes_like(
                "intermediate cache with cold root",
                &archived,
                &expected,
                4,
            );
        }

        #[test]
        fn valid_cold_and_warmed_archives_preserve_operations() {
            let cold = valid_two_leaf_tree();
            let warmed = cold.clone();
            let expected =
                operation_results_without_unwind("valid tree", &warmed, 4);

            assert_eq!(expected.0, 42);
            assert!(expected.1.is_some_and(|opening| {
                *opening.root() == 42 && opening.verify(20)
            }));
            assert_eq!(expected.2, (42, 1));
            assert_eq!(expected.3, [20, 22]);

            for (case, tree) in [
                ("valid cold archive", &cold),
                ("valid warmed archive", &warmed),
            ] {
                let decoded = decode_without_unwind(case, tree);
                assert!(!decoded.root.has_cached_item());
                assert_eq!(
                    operation_results_without_unwind(case, &decoded, 4),
                    expected
                );
            }
        }

        #[test]
        fn itemless_archived_leaf_is_rejected() {
            assert_rejected_without_unwind(
                "item-less leaf",
                &tree_with_itemless_leaf(),
            );
        }

        #[test]
        fn itemless_archived_leaf_is_rejected_after_cache_warming() {
            let tree = tree_with_itemless_leaf();

            assert_eq!(*tree.root(), u8::EMPTY_SUBTREE);
            let (smallest, height) = tree.smallest_subtree();
            assert_eq!(*smallest, u8::EMPTY_SUBTREE);
            assert_eq!(height, 1);
            drop(smallest);
            assert!(tree.opening(POSITION).is_none());
            assert!(tree.walk(|_| true).next().is_none());

            assert_rejected_without_unwind("warmed item-less leaf", &tree);
        }

        #[test]
        fn inconsistent_archives_are_rejected() {
            let mut missing_root = populated_tree();
            let (child_index, _) =
                Node::<u8, H, A>::child_location(0, POSITION);
            missing_root.root.children[child_index] = None;

            let mut missing_mid = populated_tree();
            let (child_index, child_position) =
                Node::<u8, H, A>::child_location(0, POSITION);
            let child = missing_mid.root.children[child_index]
                .as_mut()
                .expect("The inserted item must populate the root child");
            let (child_index, _) =
                Node::<u8, H, A>::child_location(1, child_position);
            child.children[child_index] = None;

            let mut out_of_capacity = populated_tree();
            out_of_capacity.positions.insert(1 << 34);

            let mut unrecorded = populated_tree();
            unrecorded.positions.remove(&POSITION);

            let mut dangling = SumTree::new();
            dangling.root.children[0] = Some(Box::new(Node::new()));

            for (case, tree) in [
                ("missing root path", missing_root),
                ("missing mid-path", missing_mid),
                ("out-of-capacity position", out_of_capacity),
                ("unrecorded leaf", unrecorded),
                ("path without a leaf", dangling),
            ] {
                assert_rejected_without_unwind(case, &tree);
            }
        }
    }
}
