use std::ops::Deref;

use azuki_syntax::{ast::IfStmt, ast::Program, visitor::AstVisitor};
use azuki_tac as tac;
use bit_set::BitSet;
use tac::{BasicBlock, BinaryInst, Inst, InstKind, OpRef, TacFunc, Value};

fn compile(tac: &Program) {}

struct FuncCompiler {
    func_builder: tac::builder::FuncBuilder,

    /// Sealed basic blocks.
    ///
    /// Sealed basic blocks have all their predecessors determined.
    sealed_bbs: BitSet,

    /// Filled basic blocks.
    ///
    /// Filled basic blocks have finished filling in calculation instructions.
    filled_bbs: BitSet,
}

impl FuncCompiler {
    pub fn mark_sealed(&mut self, bb_id: usize) {
        self.sealed_bbs.insert(bb_id);
    }

    pub fn mark_filled(&mut self, bb_id: usize) {
        self.filled_bbs.insert(bb_id);
    }

    pub fn is_sealed(&self, bb_id: usize) -> bool {
        self.sealed_bbs.contains(bb_id)
    }

    pub fn is_filled(&self, bb_id: usize) -> bool {
        self.filled_bbs.contains(bb_id)
    }
}

fn empty_jump_target(bb_id: usize) -> tac::JumpTarget {
    tac::JumpTarget {
        bb: bb_id,
        params: vec![],
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct OpRefWrapper(pub OpRef);

impl Deref for OpRefWrapper {
    type Target = OpRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for OpRefWrapper {
    fn default() -> Self {
        Self(OpRef::from_raw_parts(usize::max_value(), u64::max_value()))
    }
}

impl From<OpRef> for OpRefWrapper {
    fn from(x: OpRef) -> Self {
        Self(x)
    }
}

impl FuncCompiler {}

// This implementation is the main tac-generation part.
//
// I try to use the method in https://pp.ipd.kit.edu/uploads/publikationen/braun13cc.pdf
// to directly generate SSA code from AST.
impl AstVisitor for FuncCompiler {
    type LExprResult = ();

    type ExprResult = OpRefWrapper;

    type TyResult = ();

    type StmtResult = usize;

    type ProgramResult = ();

    type FuncResult = ();

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::StmtResult {
        self.visit_expr(&stmt.cond);

        self.mark_filled(self.func_builder.curr_bb());
        self.mark_sealed(self.func_builder.curr_bb());

        let if_bb = self.func_builder.new_bb();
        self.func_builder.set_current_bb(if_bb).unwrap();
        let if_end_bb = self.visit_block_stmt(&stmt.if_block);

        let next_bb = self.func_builder.new_bb();

        self.func_builder.insert_after_current_place(Inst {
            kind: InstKind::Jump(empty_jump_target(next_bb)),
            ty: tac::Ty::Unit,
        });

        self.mark_filled(if_end_bb);
        self.mark_sealed(if_end_bb);

        match &stmt.else_block {
            azuki_syntax::ast::IfElseBlock::None => None,
            azuki_syntax::ast::IfElseBlock::If(i) => {
                let else_bb = self.func_builder.new_bb();
                self.func_builder.set_current_bb(else_bb).unwrap();
                let else_end = self.visit_if_stmt(&i);
                self.func_builder.insert_after_current_place(Inst {
                    kind: InstKind::Jump(empty_jump_target(next_bb)),
                    ty: tac::Ty::Unit,
                });
                self.mark_filled(else_end);
                self.mark_sealed(else_end);
                Some(else_end)
            }
            azuki_syntax::ast::IfElseBlock::Block(b) => {
                let else_bb = self.func_builder.new_bb();
                self.func_builder.set_current_bb(else_bb).unwrap();
                let else_end = self.visit_block_stmt(&b);
                self.func_builder.insert_after_current_place(Inst {
                    kind: InstKind::Jump(empty_jump_target(next_bb)),
                    ty: tac::Ty::Unit,
                });
                self.mark_filled(else_end);
                self.mark_sealed(else_end);
                Some(else_end)
            }
        };

        self.func_builder.set_current_bb(next_bb).unwrap();

        next_bb
    }
}
