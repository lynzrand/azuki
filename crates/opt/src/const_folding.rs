use azuki_tac::{
    builder::FuncEditor, optimizer::FunctionOptimizer, BinaryInst, BinaryOp, Inst, InstId,
    InstKind, TacFunc, Ty, Value,
};
use smallvec::SmallVec;
use tracing::{debug, debug_span, info_span, trace, trace_span};

pub struct ConstFolding {}

impl ConstFolding {
    pub fn new() -> Self {
        Self {}
    }
}

impl FunctionOptimizer for ConstFolding {
    fn name(&self) -> std::borrow::Cow<str> {
        "const-folding".into()
    }

    fn edits_program(&self) -> bool {
        true
    }

    fn optimize_func(
        &mut self,
        _env: &mut azuki_tac::optimizer::OptimizeEnvironment,
        func: &mut azuki_tac::TacFunc,
    ) {
        let _span = debug_span!("const_folding", %func.name).entered();

        if func.first_block.is_none() {
            debug!("Empty function");
            return;
        }
        debug!("Parsing function");

        // In most cases, applying constant folding for one time is enough.
        let mut cursor = FuncEditor::new(func);
        cursor.set_current_bb(cursor.func.first_block.unwrap());
        // yeah i know, do-while pattern
        while {
            while cursor.move_forward() {
                let inst = cursor.current_inst().unwrap();
                let replaced = match &inst.kind {
                    InstKind::Binary(b) => {
                        // First, we try to simplify the instruction itself
                        match eval_binary_inst(b, &cursor.func).map(InstKind::Assign) {
                            Some(x) => Some(x),
                            None => {
                                // If there's no luck, we try to simplify
                                let ty = inst.ty.clone();
                                let v = eval_binary_deep(b, cursor.func);
                                match v {
                                    Some((i, n)) => {
                                        if let Some(n) = n {
                                            let idx = cursor
                                                .insert_before_current_place(Inst { kind: n, ty });
                                            cursor.func.inst_set_before(
                                                cursor.current_idx().unwrap(),
                                                idx,
                                            );
                                        }
                                        Some(i)
                                    }
                                    None => None,
                                }
                            }
                        }
                    }
                    InstKind::Assign(t) => Some(InstKind::Assign(eval_val(*t, &cursor.func))),
                    _ => None,
                };
                if let Some(r) = replaced {
                    trace!(
                        "replaced %{} with {:?}",
                        cursor.current_idx().unwrap().slot(),
                        r
                    );
                    cursor.current_inst_mut().unwrap().kind = r;
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

/// Evaluates a binary instruction to a simple value, if possible. Returns
/// `Some(Value)` if this instruction can be reduced into an assignment, else
/// return `None`.
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

/// Evaluate binary instructions that may reduce into simpler forms.
///
/// # Returns
///
/// - `None` if no possible reduction can be found.
/// - `Some(inst1, None)` if the operation can be reduced to one instruction.
/// - `Some(inst1, Some(inst2))` if this operation can be reduced to two instructions.
///
/// An `InstId::from_bits(u64::max_value())` refers to the id of `inst2`, if exists.
///
/// # Example
///
/// ```plaintext
/// (%3 i32 add %1 1)
/// (%4 i32 add %2 2)
/// (%5 i32 add %3 %4)
/// ```
///
/// Can be reduced into
///
/// ```plaintext
/// (%6 i32 add %1 %2)
/// (%5 i32 add %6 3)
/// ```
fn eval_binary_deep(binary: &BinaryInst, f: &TacFunc) -> Option<(InstKind, Option<InstKind>)> {
    if !is_additive(binary.op) {
        return None;
    }

    // Check if there's any possible combination that can be optimized.
    // Gets all operands of this operation.
    let mut operands = SmallVec::<[_; 4]>::new();

    match binary.lhs {
        Value::Dest(i) => match &f.inst_get(i).kind {
            InstKind::Binary(b) if is_additive(b.op) => {
                operands.push((false, b.lhs));
                operands.push((b.op == BinaryOp::Sub, b.rhs));
            }
            _ => operands.push((false, Value::Dest(i))),
        },
        i @ Value::Imm(_) => operands.push((false, i)),
    };
    let is_sub = binary.op == BinaryOp::Sub;
    match binary.rhs {
        Value::Dest(i) => match &f.inst_get(i).kind {
            InstKind::Binary(b) if is_additive(b.op) => {
                operands.push((is_sub, b.lhs));
                operands.push((is_sub ^ (b.op == BinaryOp::Sub), b.rhs));
            }
            _ => operands.push((is_sub, Value::Dest(i))),
        },
        i @ Value::Imm(_) => operands.push((is_sub, i)),
    };

    let constant = operands
        .iter()
        .filter_map(|(is_neg, v)| v.get_imm().map(|x| (*is_neg, x)))
        .fold(0i64, |acc, (is_neg, v)| {
            if is_neg {
                acc.wrapping_sub(v)
            } else {
                acc.wrapping_add(v)
            }
        });

    let mut variables = operands
        .iter()
        .filter_map(|(is_neg, v)| v.get_inst().map(|x| (*is_neg, x)))
        .collect::<SmallVec<[_; 4]>>();

    if variables.len() == 4 {
        // "a + b + c + d" type, No way to reduce.
        None
    } else if variables.len() == 3 {
        // "a + b + c + constant" type. if constant == 0 and a positive term
        // exists, we can reduce that to "pos +/- (b + c)"
        let first_positive_term = variables
            .iter()
            .enumerate()
            .find_map(|x| (!x.1 .0).then(|| x.0));

        if constant == 0 && first_positive_term.is_some() {
            let (_, pos_term) = variables.remove(first_positive_term.unwrap());
            let op = ((variables[0].0) ^ (variables[1].0))
                .then(|| BinaryOp::Add)
                .unwrap_or(BinaryOp::Sub);

            let second_inst = InstKind::Binary(BinaryInst {
                op,
                lhs: variables[0].1.into(),
                rhs: variables[1].1.into(),
            });

            let op = variables[0]
                .0
                .then(|| BinaryOp::Add)
                .unwrap_or(BinaryOp::Sub);
            let first_inst = InstKind::Binary(BinaryInst {
                op,
                lhs: Value::Dest(pos_term),
                rhs: Value::Dest(InstId::from_bits(u64::max_value())),
            });

            Some((first_inst, Some(second_inst)))
        } else {
            None
        }
    } else if variables.len() == 2 {
        // "(a + b) + constant" type
        if constant == 0 && variables.iter().any(|x| !x.0) {
            let op = ((variables[0].0) ^ (variables[1].0))
                .then(|| BinaryOp::Add)
                .unwrap_or(BinaryOp::Sub);
            let inst = InstKind::Binary(BinaryInst {
                op,
                lhs: variables[0].1.into(),
                rhs: variables[1].1.into(),
            });

            Some((inst, None))
        } else {
            let op = ((variables[0].0) ^ (variables[1].0))
                .then(|| BinaryOp::Add)
                .unwrap_or(BinaryOp::Sub);
            let second_inst = InstKind::Binary(BinaryInst {
                op,
                lhs: variables[0].1.into(),
                rhs: variables[1].1.into(),
            });

            let op = variables[0]
                .0
                .then(|| BinaryOp::Add)
                .unwrap_or(BinaryOp::Sub);
            let first_inst = InstKind::Binary(BinaryInst {
                op,
                lhs: Value::Imm(constant),
                rhs: Value::Dest(InstId::from_bits(u64::max_value())),
            });

            Some((first_inst, Some(second_inst)))
        }
    } else if variables.len() == 1 {
        // "a + constant" type

        if constant == 0 && variables[0].0 {
            Some((InstKind::Assign(Value::Dest(variables[0].1)), None))
        } else {
            let op = variables[0]
                .0
                .then(|| BinaryOp::Add)
                .unwrap_or(BinaryOp::Sub);
            let first_inst = InstKind::Binary(BinaryInst {
                op,
                lhs: Value::Imm(constant),
                rhs: variables[1].1.into(),
            });

            Some((first_inst, None))
        }
    } else if variables.is_empty() {
        Some((InstKind::Assign(Value::Imm(constant)), None))
    } else {
        unreachable!("We've covered all possible conditions")
    }
}

fn is_additive(op: BinaryOp) -> bool {
    op == BinaryOp::Add || op == BinaryOp::Sub
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
        (fn return_three () i32 
        (bb0 (
            (%0 i32 1)
            (%1 i32 mul 2 %0))
            (%2 i32 add %1 1)
            (return %2))
        )
        ";
    }
}
