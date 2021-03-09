use azuki_tac::OpRef;

pub enum Value {
    Int(i64),
    StackPointer {
        frame: u32,
        value: OpRef,
        offset: u32,
    },
    HeapPointer {
        
    },
}
