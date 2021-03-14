//! Some random linked list traits, may or may not be used in actual program.

mod implementation;

/// An implicitly linked list, allowing freestanding items and multiple links
/// inside itself.
pub trait ImplicitLinkedList {
    type Key: Copy + PartialEq;
    type Item: ImplicitLinkedListItem<Key = Self::Key>;

    fn _get_item(&self, key: Self::Key) -> &Self::Item;
    fn _get_item_mut(&mut self, key: Self::Key) -> &mut Self::Item;
    fn _insert_item(&mut self, item: Self::Item) -> Self::Key;
    fn _remove_item(&mut self, idx: Self::Key) -> Self::Item;

    fn get_item<K: Into<Self::Key>>(&self, key: K) -> &Self::Item {
        self._get_item(key.into())
    }

    fn get_item_mut<K: Into<Self::Key>>(&mut self, key: K) -> &mut Self::Item {
        self._get_item_mut(key.into())
    }

    fn insert_item<K: From<Self::Key>>(&mut self, item: Self::Item) -> K {
        self._insert_item(item).into()
    }

    fn remove_item<K: Into<Self::Key>>(&mut self, idx: K) -> Self::Item {
        self._remove_item(idx.into())
    }

    /// Position this item after the given item.
    fn attach_after(&mut self, after: impl Into<Self::Key>, this: impl Into<Self::Key>) {
        let after = after.into();
        let this = this.into();

        debug_assert!(
            self.get_item(this).is_freestanding(),
            "The value attached should be freestanding"
        );

        let after_item = self._get_item_mut(after);
        let next = after_item.next();
        after_item.set_next(Some(this));

        let current = self._get_item_mut(this);
        current.set_prev(Some(after));
        current.set_next(next);

        if let Some(idx) = next {
            let next = self._get_item_mut(idx);
            next.set_prev(Some(this));
        };
    }

    /// Position this item before the given item.
    fn attach_before(&mut self, before: impl Into<Self::Key>, this: impl Into<Self::Key>) {
        let before = before.into();
        let this = this.into();

        debug_assert!(
            self.get_item(this).is_freestanding(),
            "The value attached should be freestanding"
        );

        let before_item = self._get_item_mut(before);
        let prev = before_item.prev();
        before_item.set_next(Some(this));

        let current = self._get_item_mut(this);
        current.set_next(Some(before));
        current.set_prev(prev);

        if let Some(idx) = prev {
            let prev = self._get_item_mut(idx);
            prev.set_next(Some(this));
        };
    }

    /// Detaches this item from the list.
    fn detach(&mut self, idx: impl Into<Self::Key>) {
        let idx = idx.into();

        let inst = self._get_item_mut(idx);
        let next_idx = inst.take_next();
        let prev_idx = inst.take_prev();

        if let Some(prev_idx) = prev_idx {
            let prev = self._get_item_mut(prev_idx);
            prev.set_next(next_idx);
        }
        if let Some(next_idx) = next_idx {
            let next = self._get_item_mut(next_idx);
            next.set_prev(prev_idx)
        }
    }

    /// Split the chain into two after the given item, or return `None` if no
    /// item is after the given item.
    fn split_after(&mut self, idx: impl Into<Self::Key>) -> Option<Self::Key> {
        let head = self.get_item_mut(idx).take_next();
        if let Some(head) = head {
            self._get_item_mut(head).set_prev(None);
        }
        head
    }

    /// Split the chain into two before the given item, or return `None` if no
    /// item is before the given item.
    fn split_before(&mut self, idx: impl Into<Self::Key>) -> Option<Self::Key> {
        let tail = self.get_item_mut(idx).take_prev();
        if let Some(tail) = tail {
            self._get_item_mut(tail).set_next(None);
        }
        tail
    }

    fn connect(&mut self, tail: impl Into<Self::Key>, head: impl Into<Self::Key>) {
        let tail = tail.into();
        let head = head.into();

        debug_assert!(head != tail, "Cannot connect an item to itself");
        debug_assert!(
            self.get_item(head).prev().is_none(),
            "Head item should be the last one in chain"
        );
        debug_assert!(
            self.get_item(tail).next().is_none(),
            "Tail item should be the first one in chain"
        );

        let head_item = self.get_item_mut(head);
        head_item.set_prev(Some(tail));
        let tail_item = self.get_item_mut(tail);
        tail_item.set_next(Some(head));
    }
}

/// An implicit linked list item. Contains keys for the previous and next items
/// of `Self`.
pub trait ImplicitLinkedListItem {
    type Key: Copy + PartialEq;

    fn next(&self) -> Option<Self::Key>;
    fn set_next(&mut self, key: Option<Self::Key>);
    fn take_next(&mut self) -> Option<Self::Key> {
        let next = self.next();
        self.set_next(None);
        next
    }

    fn prev(&self) -> Option<Self::Key>;
    fn set_prev(&mut self, key: Option<Self::Key>);
    fn take_prev(&mut self) -> Option<Self::Key> {
        let prev = self.prev();
        self.set_prev(None);
        prev
    }

    fn is_freestanding(&self) -> bool {
        self.next().is_none() && self.prev().is_none()
    }
}

pub struct Cursor<'a, Ctx> {
    ctx: &'a mut Ctx,
}
