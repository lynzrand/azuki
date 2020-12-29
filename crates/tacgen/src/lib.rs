use std::ops::Deref;

use azuki_syntax::{ast::*, visitor::AstVisitor};
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

    fn mark_current_bb_as_sealed(&mut self) {
        self.mark_sealed(self.func_builder.curr_bb());
    }

    fn mark_current_bb_as_filled(&mut self) {
        self.mark_filled(self.func_builder.curr_bb());
    }
}

fn empty_jump_target(bb_id: usize) -> tac::JumpTarget {
    tac::JumpTarget {
        bb: bb_id,
        params: vec![],
    }
}

// This implementation is the main tac-generation part.
//
// I try to use the method in https://pp.ipd.kit.edu/uploads/publikationen/braun13cc.pdf
// to directly generate SSA code from AST.
impl AstVisitor for FuncCompiler {
    type LExprResult = ();

    type ExprResult = Value;

    type TyResult = ();

    type StmtResult = ();

    type ProgramResult = ();

    type FuncResult = ();

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::StmtResult {
        let expr_val = self.visit_expr(&stmt.cond);

        // TODO: add conditional jump instruction

        self.mark_current_bb_as_filled();
        self.mark_current_bb_as_sealed();

        // Create if block
        let if_bb = self.func_builder.new_bb();
        self.func_builder.set_current_bb(if_bb).unwrap();

        self.visit_block_stmt(&stmt.if_block);
        let if_end_bb = self.func_builder.curr_bb();

        self.mark_current_bb_as_filled();
        self.mark_current_bb_as_sealed();

        // Deal with else block
        let else_end_bb = match &stmt.else_block {
            azuki_syntax::ast::IfElseBlock::None => None,
            other => {
                let else_bb = self.func_builder.new_bb();
                self.func_builder.set_current_bb(else_bb).unwrap();

                match other {
                    IfElseBlock::None => unreachable!(),
                    IfElseBlock::If(i) => self.visit_if_stmt(&i),
                    IfElseBlock::Block(b) => self.visit_block_stmt(&b),
                }

                self.mark_current_bb_as_filled();
                self.mark_current_bb_as_sealed();

                Some(self.func_builder.curr_bb())
            }
        };

        // The basic block after the if statement
        let next_bb = self.func_builder.new_bb();

        self.func_builder
            .insert_at_end_of(
                Inst {
                    kind: InstKind::Jump(empty_jump_target(next_bb)),
                    ty: tac::Ty::Unit,
                },
                if_end_bb,
            )
            .unwrap();

        if let Some(bb) = else_end_bb {
            self.func_builder
                .insert_at_end_of(
                    Inst {
                        kind: InstKind::Jump(empty_jump_target(next_bb)),
                        ty: tac::Ty::Unit,
                    },
                    bb,
                )
                .unwrap();
        }

        self.func_builder.set_current_bb(next_bb).unwrap();
    }
}
