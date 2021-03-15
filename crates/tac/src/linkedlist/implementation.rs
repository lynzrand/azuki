use thunderdome::{Arena, Index};

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

impl<T, Key> ImplicitLinkedList<Key> for Arena<T>
where
    T: ImplicitLinkedListItem<Key = Key>,
    Key: Copy + Into<Index> + From<Index> + Eq,
{
    type Item = T;

    fn get_item(&self, key: Key) -> &Self::Item {
        &self[key.into()]
    }

    fn get_item_mut(&mut self, key: Key) -> &mut Self::Item {
        &mut self[key.into()]
    }

    fn insert_item(&mut self, item: Self::Item) -> Key {
        self.insert(item).into()
    }

    fn remove_item(&mut self, idx: Key) -> Self::Item {
        self.remove(idx.into()).unwrap()
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
