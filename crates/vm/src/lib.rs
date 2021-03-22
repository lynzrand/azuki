use std::{cell::RefCell, collections::HashMap, rc::Rc};

use azuki_tac::{BBId, BinaryInst, Inst, InstId, Program, TacFunc, Value};
use inspector::Inspector;

pub mod inspector;
mod test;
pub mod value;

pub struct Vm<'src> {
    program: &'src Program,
    stack: Vec<Frame<'src>>,
    inspectors: Vec<Rc<RefCell<dyn Inspector>>>,
}

pub struct Frame<'f> {
    func: &'f TacFunc,
    last_bb: BBId,
    bb: BBId,
    params: Vec<i64>,
    instruction: CurrInst,
    vars: HashMap<InstId, i64>,
}

impl<'f> Frame<'f> {
    pub fn eval(&self, value: Value) -> Option<i64> {
        match value {
            Value::Dest(d) => self.vars.get(&d).cloned(),
            Value::Imm(i) => Some(i),
        }
    }

    pub fn move_to(&mut self, bb: BBId) {
        self.bb = bb;
        self.instruction = self.func.bb_get(bb).head.into();
    }

    /// Get a reference to the frame's bb.
    pub fn bb(&self) -> &BBId {
        &self.bb
    }

    /// Get a reference to the frame's last bb.
    pub fn last_bb(&self) -> &BBId {
        &self.last_bb
    }

    /// Get a reference to the frame's func.
    pub fn func(&self) -> &'f TacFunc {
        self.func
    }

    /// Get a reference to the frame's instruction.
    pub fn instruction(&self) -> Option<&Inst> {
        match self.instruction {
            CurrInst::Instruction(i) => Some(&self.func.tac_get(i).inst),
            CurrInst::Jump => None,
        }
    }

    /// Get a reference to the frame's vars.
    pub fn vars(&self) -> &HashMap<InstId, i64> {
        &self.vars
    }
}

enum CurrInst {
    Instruction(InstId),
    Jump,
}

impl From<Option<InstId>> for CurrInst {
    fn from(x: Option<InstId>) -> Self {
        match x {
            Some(o) => Self::Instruction(o),
            None => Self::Jump,
        }
    }
}

enum JumpAction {
    Goto(BBId),
    Return(Option<Value>),
    Error,
}

impl<'src> Vm<'src> {
    pub fn new(program: &'src Program) -> Vm<'src> {
        Vm {
            program,
            stack: Vec::new(),
            inspectors: Vec::new(),
        }
    }

    pub fn add_inspector_boxed(&mut self, inspector: Rc<RefCell<dyn Inspector>>) {
        self.inspectors.push(inspector);
    }

    pub fn run_func(&mut self, name: &str, params: Vec<i64>) -> Option<i64> {
        let func = self
            .program
            .functions
            .get(name)
            .expect("Function does not exist");

        self.inspectors
            .iter_mut()
            .for_each(|i| i.borrow_mut().before_call(&params, func));

        self.stack.push(Frame {
            func,
            instruction: func
                .bb_get(func.starting_block().unwrap())
                .head
                .map_or(CurrInst::Jump, CurrInst::Instruction),
            params,
            vars: HashMap::new(),
            last_bb: BBId::default(),
            bb: func.starting_block().unwrap(),
        });

        let ret = self.run_till_return()?;

        self.stack.pop();
        Some(ret)
    }

    fn run_till_return(&mut self) -> Option<i64> {
        assert!(!self.stack.is_empty());
        loop {
            let last = self.stack.last_mut().unwrap();
            match last.instruction {
                CurrInst::Instruction(i) => {
                    let next = last.func.tac_get(i).next;
                    self.run_inst_in_curr_func(i);
                    let last = self.stack.last_mut().unwrap();
                    last.instruction = next.into();
                }
                CurrInst::Jump => {
                    if let Some(value) = self.run_jump_inst() {
                        return value;
                    }
                }
            }
        }
    }

    fn run_inst_in_curr_func(&mut self, idx: InstId) {
        assert!(!self.stack.is_empty());

        let last = self.stack.last().unwrap();
        let inst = last.func.tac_get(idx);

        self.inspectors
            .iter_mut()
            .for_each(|i| i.borrow_mut().before_inst(&inst.inst, last));

        let res = match &inst.inst.kind {
            azuki_tac::InstKind::Binary(bin) => self.run_binary_inst(last, bin),
            azuki_tac::InstKind::FunctionCall(func) => {
                let params = func
                    .params
                    .iter()
                    .map(|x| last.eval(*x))
                    .collect::<Option<Vec<_>>>()
                    .unwrap();

                self.run_func(&func.name, params)
            }
            azuki_tac::InstKind::Assign(v) => last.eval(*v),
            azuki_tac::InstKind::Phi(sources) => {
                let last_bb = last.last_bb;
                sources.get(&last_bb).and_then(|&val| last.eval(val.into()))
            }
            azuki_tac::InstKind::Param(i) => last.params.get(*i).cloned(),
            azuki_tac::InstKind::Dead => None,
        };

        let last = self.stack.last_mut().unwrap();
        last.vars.insert(idx, res.unwrap());
    }

    fn run_binary_inst(&self, frame: &Frame, inst: &BinaryInst) -> Option<i64> {
        let lhs = frame.eval(inst.lhs)?;
        let rhs = frame.eval(inst.rhs)?;
        let res = match inst.op {
            azuki_tac::BinaryOp::Add => lhs + rhs,
            azuki_tac::BinaryOp::Sub => lhs - rhs,
            azuki_tac::BinaryOp::Mul => lhs * rhs,
            azuki_tac::BinaryOp::Div => lhs.checked_div(rhs)?,
            azuki_tac::BinaryOp::Lt => (lhs < rhs) as i64,
            azuki_tac::BinaryOp::Gt => (lhs > rhs) as i64,
            azuki_tac::BinaryOp::Le => (lhs <= rhs) as i64,
            azuki_tac::BinaryOp::Ge => (lhs >= rhs) as i64,
            azuki_tac::BinaryOp::Eq => (lhs == rhs) as i64,
            azuki_tac::BinaryOp::Ne => (lhs != rhs) as i64,
        };
        Some(res)
    }

    fn run_jump_inst(&mut self) -> Option<Option<i64>> {
        let last = self.stack.last_mut().unwrap();

        last.last_bb = last.bb;
        let mut action = JumpAction::Error;
        for inst in &last.func.bb_get(last.bb).branch {
            self.inspectors
                .iter_mut()
                .for_each(|i| i.borrow_mut().before_branch(inst, last));

            match inst {
                azuki_tac::Branch::Return(v) => {
                    action = JumpAction::Return(*v);
                    break;
                }
                azuki_tac::Branch::Jump(target) => {
                    action = JumpAction::Goto(*target);
                    break;
                }
                azuki_tac::Branch::CondJump {
                    cond,
                    if_true: target,
                } => {
                    if last.eval(*cond).map_or(false, |x| x != 0) {
                        action = JumpAction::Goto(*target);
                        break;
                    }
                }
            }
        }
        match action {
            JumpAction::Goto(bb) => last.move_to(bb),
            JumpAction::Return(v) => {
                self.inspectors
                    .iter_mut()
                    .for_each(|i| i.borrow_mut().before_ret(&last));
                return Some(v.and_then(|v| last.eval(v)));
            }
            JumpAction::Error => {
                panic!("Error")
            }
        }
        None
    }
}
