// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::cell::Ref;

use crate::{Aggregate, Node, Tree};

/// Iterator that walks through a tree's leaves, according to a walker function.
#[derive(Debug, Clone)]
pub struct Walk<'a, T, W, const H: usize, const A: usize> {
    root: &'a Node<T, H, A>,
    walker: W,

    // These boots are made for walkin'.
    path: [Option<&'a Node<T, H, A>>; H],
    indices: [usize; H],
}

impl<'a, T, W, const H: usize, const A: usize> Walk<'a, T, W, H, A>
where
    T: Aggregate<A>,
    W: Fn(&T) -> bool,
{
    pub(crate) fn new(tree: &'a Tree<T, H, A>, walker: W) -> Self {
        Self {
            root: &tree.root,
            walker,
            path: [None; H],
            indices: [0; H],
        }
    }

    /// Advances the iterator recursively, returning a new leaf node if it is
    /// found.
    pub(crate) fn advance(
        &mut self,
        node: &'a Node<T, H, A>,
        h: usize,
    ) -> Option<Ref<'a, T>> {
        // We are at a node before a leaf, therefore we should try to return our
        // first eligible child.
        if h == H - 1 {
            let index = &mut self.indices[h];

            // We keep iterating the stored index to ensure that when/if we
            // return to this child we start from the previous index.
            for i in *index..A {
                *index = i + 1;
                if let Some(leaf) = &node.children[i] {
                    let leaf = leaf.item();
                    if (self.walker)(&*leaf) {
                        return Some(leaf);
                    }
                }
            }

            // We will never return here, so we should set this to zero to
            // ensure our siblings start looking at their first child.
            *index = 0;
            return None;
        }

        // If there is no child in the path, we have never been here before.
        // Therefore we try to set the path to one of our children, starting
        // from the first.
        if self.path[h].is_none() {
            for i in 0..A {
                self.indices[h] = i;
                if let Some(child) = &node.children[i] {
                    let child = child.as_ref();
                    if (self.walker)(&*child.item()) {
                        self.path[h] = Some(child);
                        break;
                    }
                }
            }
        }

        // If there is a child in the path, we have either set it previously or
        // are re-entering. Either way we should advance.
        //
        // If the advance returns `Some` we just return the item, otherwise we
        // try the next child in line, and advance through it.
        if let Some(child) = self.path[h] {
            if let Some(item) = self.advance(child, h + 1) {
                return Some(item);
            }

            for i in self.indices[h] + 1..A {
                self.indices[h] = i;

                if let Some(child) = &node.children[i] {
                    let child = child.as_ref();
                    if (self.walker)(&*child.item()) {
                        self.path[h] = Some(child);
                        if let Some(item) = self.advance(child, h + 1) {
                            return Some(item);
                        }
                    }
                }
            }

            self.path[h] = None;
            self.indices[h] = 0;
        }

        None
    }
}

impl<'a, T, W, const H: usize, const A: usize> Iterator for Walk<'a, T, W, H, A>
where
    T: Aggregate<A>,
    W: Fn(&T) -> bool,
{
    type Item = Ref<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.advance(self.root, 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Aggregate, Tree};

    #[derive(Debug, Default, Clone, Copy)]
    struct Max(u64);

    impl From<u64> for Max {
        fn from(i: u64) -> Self {
            Max(i)
        }
    }

    const HEIGHT_2: usize = 2;
    const HEIGHT_17: usize = 17;

    const ARITY_2: usize = 2;
    const ARITY_4: usize = 4;

    const LARGER_THAN: u64 = 6;

    impl<const A: usize> Aggregate<A> for Max {
        const EMPTY_SUBTREE: Self = Max(0);

        fn aggregate(items: [&Self; A]) -> Self {
            Self(items.into_iter().map(|i| i.0).max().unwrap_or_default())
        }
    }

    type SmallTree = Tree<Max, HEIGHT_2, ARITY_2>;
    type LargeTree = Tree<Max, HEIGHT_17, ARITY_4>;

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_larger_than(max: &Max) -> bool {
        max.0 > LARGER_THAN
    }

    #[test]
    fn full_tree() {
        let mut tree = SmallTree::new();

        tree.insert(0, 2);
        tree.insert(1, 8);
        tree.insert(2, 16);
        tree.insert(3, 4);

        let mut walk = tree.walk(is_larger_than);

        assert!(matches!(walk.next(), Some(x) if x.0 == 8));
        assert!(matches!(walk.next(), Some(x) if x.0 == 16));
        assert!(matches!(walk.next(), None));
    }

    #[test]
    fn partial_tree() {
        let mut tree = SmallTree::new();

        tree.insert(1, 8);
        tree.insert(3, 4);

        let mut walk = tree.walk(is_larger_than);

        assert!(matches!(walk.next(), Some(x) if x.0 == 8));
        assert!(matches!(walk.next(), None));
    }

    #[test]
    fn large_tree() {
        let mut tree = LargeTree::new();

        tree.insert(0x42, 16);
        tree.insert(0x666, 1);
        tree.insert(0x1ead, 25);
        tree.insert(0xbeef, 8);
        tree.insert(0xca11, 25);
        tree.insert(0xdead, 4);

        let mut walk = tree.walk(is_larger_than);

        assert!(matches!(walk.next(), Some(x) if x.0 == 16));
        assert!(matches!(walk.next(), Some(x) if x.0 == 25));
        assert!(matches!(walk.next(), Some(x) if x.0 == 8));
        assert!(matches!(walk.next(), Some(x) if x.0 == 25));
        assert!(matches!(walk.next(), None));
    }

    #[test]
    fn empty_tree() {
        let tree = SmallTree::new();
        let mut walk = tree.walk(is_larger_than);
        assert!(matches!(walk.next(), None));
    }
}
