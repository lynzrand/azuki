use crate::{ImplicitLinkedList, ImplicitLinkedListItem};

struct ExampleItem(
    i64,
    Option<slotmap::DefaultKey>,
    Option<slotmap::DefaultKey>,
);

impl ImplicitLinkedListItem for ExampleItem {
    type Key = slotmap::DefaultKey;

    fn next(&self) -> Option<Self::Key> {
        self.2
    }

    fn set_next(&mut self, key: Option<Self::Key>) {
        self.2 = key
    }

    fn prev(&self) -> Option<Self::Key> {
        self.1
    }

    fn set_prev(&mut self, key: Option<Self::Key>) {
        self.1 = key
    }
}

#[test]
fn linked_list_connect() {
    let mut list = slotmap::SlotMap::new();
    let one = list.insert(ExampleItem(1, None, None));
    let two = list.insert(ExampleItem(2, None, None));
    list.connect(one, two);
    assert_eq!(list.get(one).unwrap().next(), Some(two));
    assert_eq!(list.get(two).unwrap().prev(), Some(one));
    let three = list.insert(ExampleItem(3, None, None));
    list.attach_after(one, three);
    assert_eq!(list.get(one).unwrap().next(), Some(three));
    assert_eq!(list.get(two).unwrap().prev(), Some(three));
}
