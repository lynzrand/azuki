use slotmap::SlotMap;

use crate::Tac;
use crate::{BBId, BasicBlock, InstId};

use super::{ImplicitLinkedList, ImplicitLinkedListItem};

impl ImplicitLinkedListItem for Tac {
    type Key = InstId;

    fn next(&self) -> Option<Self::Key> {
        self.next
    }

    fn set_next(&mut self, key: Option<Self::Key>) {
        self.next = key
    }

    fn prev(&self) -> Option<Self::Key> {
        self.prev
    }

    fn set_prev(&mut self, key: Option<Self::Key>) {
        self.prev = key
    }

    fn take_next(&mut self) -> Option<Self::Key> {
        self.next.take()
    }

    fn take_prev(&mut self) -> Option<Self::Key> {
        self.prev.take()
    }
}

impl<T, Key> ImplicitLinkedList<Key> for SlotMap<Key, T>
where
    T: ImplicitLinkedListItem<Key = Key>,
    Key: Copy + Eq + slotmap::Key,
{
    type Item = T;

    fn get_item(&self, key: Key) -> &Self::Item {
        &self[key]
    }

    fn get_item_mut(&mut self, key: Key) -> &mut Self::Item {
        &mut self[key]
    }

    fn insert_item(&mut self, item: Self::Item) -> Key {
        self.insert(item)
    }

    fn remove_item(&mut self, idx: Key) -> Self::Item {
        self.remove(idx).unwrap()
    }
}

impl ImplicitLinkedListItem for BasicBlock {
    type Key = BBId;

    fn next(&self) -> Option<Self::Key> {
        self.next
    }

    fn set_next(&mut self, key: Option<Self::Key>) {
        self.next = key
    }

    fn prev(&self) -> Option<Self::Key> {
        self.prev
    }

    fn set_prev(&mut self, key: Option<Self::Key>) {
        self.prev = key
    }
}
