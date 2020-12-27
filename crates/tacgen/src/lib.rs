
use azuki_tac as tac;
use tac::{BasicBlock, BinaryInst, Inst, InstKind, TacFunc, Value};

#[test]
fn test() {
    let mut func_builder = tac::builder::FuncBuilder::new("add".into());
    let a = func_builder.insert_after_current_place(Inst {
        kind: InstKind::Param(0),
        ty: tac::Ty::Int,
    });
    let b = func_builder.insert_after_current_place(Inst {
        kind: InstKind::Param(1),
        ty: tac::Ty::Int,
    });
    let res = func_builder.insert_after_current_place(Inst {
        kind: InstKind::Binary(BinaryInst {
            op: tac::BinaryOp::Add,
            lhs: a.into(),
            rhs: b.into(),
        }),
        ty: tac::Ty::Int,
    });
    func_builder.insert_after_current_place(Inst {
        kind: InstKind::Return(res.into()),
        ty: tac::Ty::Unit,
    });
    let f = func_builder.build();
    println!("{}", f);
}
