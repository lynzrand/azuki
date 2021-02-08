//! Some random linked list traits, may or may not be used in actual program.

pub trait SinglyLinkedList {
    type Key;
    type Context;

    fn get_value(ctx: &Self::Context, key: Self::Key) -> &Self;
    fn get_value_mut(ctx: &mut Self::Context, key: Self::Key) -> &mut Self;
    fn insert_value_after(ctx: &mut Self::Context, value: Self) -> Self::Key;

    fn next_value_key(&self) -> Option<Self::Key>;
    fn set_next_value_key(&mut self);
}

pub trait DoublyLinkedList: SinglyLinkedList {
    fn prev_value_key(&self) -> Option<Self::Key>;
    fn set_prev_value_key(&mut self);
}

pub struct Cursor<'a, Ctx> {
    ctx: &'a mut Ctx,
}
