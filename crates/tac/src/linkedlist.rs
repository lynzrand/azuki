pub trait SinglyLinkedList {
    type Key;
    type Context;

    fn get_value(ctx: &Self::Context, key: Self::Key) -> &Self;
    fn get_value_mut(ctx: &mut Self::Context, key: Self::Key) -> &mut Self;

    fn next_value_key(&self) -> Option<Self::Key>;
}

pub trait DoublyLinkedList: SinglyLinkedList {
    fn prev_value_key(&self) -> Option<Self::Key>;
}

pub struct Cursor {}
