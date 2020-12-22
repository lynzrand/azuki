use azuki_tac as tac;
use tac::{BasicBlock, BinaryInst, Inst, TacFunc, Value};

#[test]
fn test() {
    let mut f = TacFunc::new();
    let start = f.tac_new(Inst::Const(123));
    f.basic_blocks.insert(
        0,
        BasicBlock {
            op_start: Some(start),
        },
    );
    let _2 = f.tac_insert_after(start, Inst::Const(456)).unwrap();
    f.tac_insert_after(
        _2,
        Inst::Binary(BinaryInst {
            op: tac::BinaryOp::Add,
            lhs: Value::Dest(start),
            rhs: Value::Dest(_2),
        }),
    )
    .unwrap();
}
