use azuki_tac::{
    builder::FuncEditor, optimizer::FunctionOptimizer, BinaryInst, BinaryOp, Inst, InstId,
    InstKind, TacFunc, Value,
};

pub struct ConstFolding {}

impl ConstFolding {
    pub fn new() -> Self {
        Self {}
    }
}

impl FunctionOptimizer for ConstFolding {
    fn name(&self) -> std::borrow::Cow<str> {
        "const_propagation".into()
    }

    fn edits_program(&self) -> bool {
        true
    }

    fn optimize_func(
        &mut self,
        _env: &mut azuki_tac::optimizer::OptimizeEnvironment,
        func: &mut azuki_tac::TacFunc,
    ) {
        if func.first_block.is_none() {
            return;
        }
        // In most cases, applying constant folding for one time is enough.
        let mut cursor = FuncEditor::new(func);
        cursor.set_current_bb(cursor.func.first_block.unwrap());
        // yeah i know, do-while pattern
        while {
            while cursor.move_forward() {
                let inst = cursor.current_inst().unwrap();
                let replaced = match &inst.kind {
                    azuki_tac::InstKind::Binary(b) => eval_binary_inst(b, &cursor.func),
                    azuki_tac::InstKind::Assign(t) => Some(eval_val(*t, &cursor.func)),
                    _ => None,
                };
                if let Some(r) = replaced {
                    cursor.current_inst_mut().unwrap().kind = InstKind::Assign(r);
                }
            }
            let next = cursor.current_bb().next;
            match next {
                Some(next) => {
                    cursor.set_current_bb(next);
                    true
                }
                None => false,
            }
        } {}
    }
}

fn eval_binary_inst(binary: &BinaryInst, f: &TacFunc) -> Option<Value> {
    use BinaryOp::*;
    use Value::*;
    let lhs = eval_val(binary.lhs, f);
    let rhs = eval_val(binary.rhs, f);
    match (binary.op, lhs, rhs) {
        // Constant op
        (op, Imm(lhs), Imm(rhs)) => eval_binary(op, lhs, rhs).map(Imm),

        // No-op
        (Add, Imm(0), v)
        | (Add, v, Imm(0))
        | (Sub, v, Imm(0))
        | (Mul, Imm(1), v)
        | (Mul, v, Imm(1))
        | (Div, v, Imm(1)) => Some(v),

        // Zero-op
        (Mul, Imm(0), _) | (Mul, _, Imm(0)) | (Div, Imm(0), _) => Some(Imm(0)),
        (Sub, Dest(a), Dest(b)) if a == b => Some(Imm(0)),

        // Divide by 0
        (Div, _, Imm(0)) => None,

        // Constant division (because div-0 is UB)
        (Div, Dest(a), Dest(b)) if a == b => Some(Imm(1)),

        // Others
        _ => None,
    }
}

fn eval_val(val: Value, f: &TacFunc) -> Value {
    match val {
        Value::Dest(inst) => eval_inst(inst, f),
        i @ Value::Imm(_) => i,
    }
}

fn eval_inst(inst: InstId, f: &TacFunc) -> Value {
    let i = f.inst_get(inst);
    if let Some(val) = i.kind.as_assign() {
        match val {
            Value::Dest(d) => eval_inst(*d, f),
            Value::Imm(i) => Value::Imm(*i),
        }
    } else {
        Value::Dest(inst)
    }
}

fn eval_binary(op: BinaryOp, lhs: i64, rhs: i64) -> Option<i64> {
    Some(match op {
        BinaryOp::Add => lhs.wrapping_add(rhs),
        BinaryOp::Sub => lhs.wrapping_sub(rhs),
        BinaryOp::Mul => lhs.wrapping_mul(rhs),
        BinaryOp::Div => lhs.checked_div(rhs)?,
        BinaryOp::Lt => (lhs < rhs) as i64,
        BinaryOp::Gt => (lhs > rhs) as i64,
        BinaryOp::Le => (lhs <= rhs) as i64,
        BinaryOp::Ge => (lhs >= rhs) as i64,
        BinaryOp::Eq => (lhs == rhs) as i64,
        BinaryOp::Ne => (lhs != rhs) as i64,
    })
}

#[cfg(test)]
mod test {
    #[test]
    fn test_const_folding() {
        let input = r"
        fn return_three() -> i32 {
        bb0:
            %0 = i32 #1
            %1 = i32 mul
        }
        ";
    }
}
