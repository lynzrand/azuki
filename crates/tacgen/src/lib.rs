pub mod err;

use azuki_syntax::{ast::*, visitor::AstVisitor};
use azuki_tac as tac;
use bit_set::BitSet;
use err::Error;
use std::{collections::BTreeMap, ops::Deref};

use tac::{BasicBlock, BinaryInst, Branch, Inst, InstKind, OpRef, TacFunc, Ty, Value};

fn compile(tac: &Program) {}

struct FuncCompiler {
    builder: tac::builder::FuncBuilder,
}

fn empty_jump_target(bb_id: usize) -> tac::BranchTarget {
    tac::BranchTarget {
        bb: bb_id,
        params: BTreeMap::new(),
    }
}

// This implementation is the main tac-generation part.
//
// I try to use the method in https://pp.ipd.kit.edu/uploads/publikationen/braun13cc.pdf
// to directly generate SSA code from AST.
//
// Notes:
//
// - All basic blocks are marked as filled and sealed when its successor is created in another
//   visitor method. Any basic block that needs special treatments (e.g. late sealing in control
//   flows) should be managed within a single visitor method.
impl AstVisitor for FuncCompiler {
    type LExprResult = ();

    type ExprResult = Result<(Value, Ty), Error>;

    type TyResult = ();

    type StmtResult = Result<(), Error>;

    type ProgramResult = ();

    type FuncResult = ();

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::StmtResult {
        let expr_val = self.visit_expr(&stmt.cond)?;
        let last_bb = self.builder.current_bb();

        self.builder.mark_sealed(last_bb);
        self.builder.mark_sealed(last_bb);

        // Create if block
        let if_bb = self.builder.new_bb();

        // todo: mark predecessor

        self.builder.set_current_bb(if_bb).unwrap();
        self.visit_block_stmt(&stmt.if_block)?;

        let if_end_bb = self.builder.current_bb();

        // Deal with else block
        let else_bbs = match &stmt.else_block {
            azuki_syntax::ast::IfElseBlock::None => None,
            other => {
                let else_bb = self.builder.new_bb();
                self.builder.set_current_bb(else_bb).unwrap();

                match other {
                    IfElseBlock::None => unreachable!(),
                    IfElseBlock::If(i) => self.visit_if_stmt(&i)?,
                    IfElseBlock::Block(b) => self.visit_block_stmt(&b)?,
                }

                Some((else_bb, self.builder.current_bb()))
            }
        };

        // The basic block after the if statement
        let next_bb = self.builder.new_bb();

        // if -> if_bb
        //  \--> else_bb / next_bb
        self.builder
            .set_jump_inst(
                Branch::CondJump {
                    cond: expr_val.0,
                    target: empty_jump_target(if_bb),
                    target_if_false: empty_jump_target(else_bbs.map(|x| x.0).unwrap_or(next_bb)),
                },
                last_bb,
            )
            .unwrap();

        // if_end_bb -> next_bb
        self.builder
            .set_jump_inst(Branch::Jump(empty_jump_target(next_bb)), if_end_bb)
            .unwrap();

        // else_end_bb -> next_bb
        if let Some((_, bb)) = else_bbs {
            self.builder
                .set_jump_inst(Branch::Jump(empty_jump_target(next_bb)), bb)
                .unwrap();
        }

        self.builder.set_current_bb(next_bb).unwrap();
        Ok(())
    }

    fn visit_while_stmt(&mut self, stmt: &WhileStmt) -> Self::StmtResult {
        let cur_bb = self.builder.current_bb();
        let cond_bb = self.builder.new_bb();
        self.builder
            .set_jump_inst(Branch::Jump(empty_jump_target(cond_bb)), cur_bb)
            .unwrap();

        self.builder.mark_sealed(cur_bb);
        self.builder.mark_filled(cur_bb);

        self.builder.set_current_bb(cond_bb).unwrap();
        let (cond, _cond_ty) = self.visit_expr(&stmt.cond)?;

        let loop_bb = self.builder.new_bb();
        let next_bb = self.builder.new_bb();

        self.builder.mark_filled(cond_bb);
        self.builder
            .set_jump_inst(
                Branch::CondJump {
                    cond,
                    target: empty_jump_target(loop_bb),
                    target_if_false: empty_jump_target(next_bb),
                },
                cond_bb,
            )
            .unwrap();

        self.builder.set_current_bb(loop_bb).unwrap();
        self.visit_block_stmt(&stmt.body)?;
        let loop_end_bb = self.builder.current_bb();

        self.builder
            .set_jump_inst(Branch::Jump(empty_jump_target(cond_bb)), loop_end_bb)
            .unwrap();

        self.builder.mark_sealed(loop_end_bb);
        self.builder.mark_filled(loop_end_bb);
        self.builder.mark_sealed(cond_bb);

        self.builder.set_current_bb(next_bb).unwrap();

        Ok(())
    }

    fn visit_literal_expr(&mut self, _expr: &LiteralExpr) -> Self::ExprResult {
        match _expr.kind {
            LiteralKind::Integer(val) => Ok((Value::Imm(val as i64), Ty::Int)),
            LiteralKind::Float(_) => {
                todo!("implement float (or not)")
            }
            LiteralKind::String(_) => {
                todo!("Implement String")
            }
            LiteralKind::Char(ch) => Ok((Value::Imm(ch as i64), Ty::Int)),
        }
    }

    fn visit_binary_expr(&mut self, expr: &BinaryExpr) -> Self::ExprResult {
        let (lhsv, lhst) = self.visit_expr(&expr.lhs)?;
        let (rhsv, rhst) = self.visit_expr(&expr.rhs)?;

        assert_type_eq(&lhst, &rhst)?;

        let v = self.builder.insert_after_current_place(Inst {
            kind: InstKind::Binary(BinaryInst {
                op: match expr.op {
                    BinaryOp::Add => tac::BinaryOp::Add,
                    BinaryOp::Sub => tac::BinaryOp::Sub,
                    BinaryOp::Mul => tac::BinaryOp::Mul,
                    BinaryOp::Div => tac::BinaryOp::Div,
                    BinaryOp::Gt => tac::BinaryOp::Gt,
                    BinaryOp::Lt => tac::BinaryOp::Lt,
                    BinaryOp::Ge => tac::BinaryOp::Ge,
                    BinaryOp::Le => tac::BinaryOp::Le,
                    BinaryOp::Eq => tac::BinaryOp::Eq,
                    BinaryOp::Neq => tac::BinaryOp::Ne,
                },
                lhs: lhsv,
                rhs: rhsv,
            }),
            ty: lhst.clone(),
        });

        Ok((v.into(), lhst))
    }

    fn visit_unary_expr(&mut self, expr: &UnaryExpr) -> Self::ExprResult {
        let (v, t) = self.visit_expr(&expr.expr)?;

        match expr.op {
            UnaryOp::Neg => {
                let v = self.builder.insert_after_current_place(Inst {
                    kind: InstKind::Binary(BinaryInst {
                        op: tac::BinaryOp::Sub,
                        lhs: Value::Imm(0),
                        rhs: v,
                    }),
                    ty: t.clone(),
                });
                Ok((v.into(), t))
            }
            UnaryOp::Pos => Ok((v, t)),
        }
    }

    fn visit_as_expr(&mut self, expr: &AsExpr) -> Self::ExprResult {
        self.visit_expr(&expr.val)
    }

    fn visit_return_stmt(&mut self, stmt: &ReturnStmt) -> Self::StmtResult {
        todo!()
    }
}

fn assert_type_eq(lhs: &Ty, rhs: &Ty) -> Result<(), err::Error> {
    if lhs != rhs {
        return Err(Error::TypeMismatch {
            expected: lhs.clone(),
            found: rhs.clone(),
        });
    }
    Ok(())
}
