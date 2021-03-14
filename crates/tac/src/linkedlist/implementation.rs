use thunderdome::{Arena, Index};

use crate::InstId;
use crate::Tac;

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

impl<T> ImplicitLinkedList for Arena<T>
where
    T: ImplicitLinkedListItem<Key = Index>,
{
    type Key = Index;

    type Item = T;

    fn _get_item(&self, key: Self::Key) -> &Self::Item {
        &self[key]
    }

    fn _get_item_mut(&mut self, key: Self::Key) -> &mut Self::Item {
        &mut self[key]
    }

    fn _insert_item(&mut self, item: Self::Item) -> Self::Key {
        self.insert(item)
    }

    fn _remove_item(&mut self, idx: Self::Key) -> Self::Item {
        self.remove(idx).unwrap()
    }
}
