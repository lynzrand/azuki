use azuki_tac::InstId;

pub enum Value {
    Int(i64),
    StackPointer {
        frame: u32,
        value: InstId,
        offset: u32,
    },
    HeapPointer {},
}
