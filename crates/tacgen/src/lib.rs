pub mod err;
pub mod symbol;
mod test;

use azuki_syntax::{ast::*, visitor::AstVisitor};
use azuki_tac as tac;
use err::Error;

use std::{cell::RefCell, collections::HashMap, rc::Rc, todo};
use symbol::{NumberingCounter, ScopeBuilder, StringInterner};

use tac::{
    builder::FuncBuilder, BBId, BinaryInst, Branch, FunctionCall, Inst, InstId, InstKind, TacFunc,
    Ty, Value,
};

pub fn compile(tac: &Program) -> Result<tac::Program, Error> {
    let interner = Rc::new(RefCell::new(StringInterner::new()));
    let counter = Rc::new(NumberingCounter::new(0));
    let global_scope_builder = Rc::new(RefCell::new(ScopeBuilder::new(counter, interner.clone())));

    let mut funcs = HashMap::new();
    for func in &tac.funcs {
        let name = func.name.name.clone();
        let mut result = TacFunc::new_untyped(name.clone());
        let mut compiler =
            FuncCompiler::new(&mut result, interner.clone(), global_scope_builder.clone());
        compiler.visit_func(func)?;
        funcs.insert(name, result);
    }
    Ok(tac::Program { functions: funcs })
}

struct BreakTarget {
    pub break_out: BBId,
    pub continue_in: BBId,
}

pub struct FuncCompiler<'a> {
    builder: tac::builder::FuncBuilder<'a, u32>,
    break_targets: Vec<BreakTarget>,

    return_ty: Ty,

    interner: Rc<RefCell<StringInterner>>,

    scope_builder: Rc<RefCell<ScopeBuilder>>,
}

impl<'a> FuncCompiler<'a> {
    pub fn new(
        func: &'a mut TacFunc,
        interner: Rc<RefCell<StringInterner>>,
        scope_builder: Rc<RefCell<ScopeBuilder>>,
    ) -> FuncCompiler<'a> {
        FuncCompiler {
            builder: FuncBuilder::new_func(func),
            break_targets: vec![],
            return_ty: Ty::unit(),
            interner,
            scope_builder,
        }
    }

    fn visit_func_param_real(
        &mut self,
        param: &FuncParam,
        idx: usize,
    ) -> Result<(InstId, Ty), Error> {
        let ty = self.visit_ty(&param.ty)?;
        let mut scope = self.scope_builder.borrow_mut();
        let var = scope
            .insert(&param.name.name, ty.clone())
            .ok_or_else(|| Error::DuplicateVar(param.name.name.clone()))?;

        let val = self.builder.insert_after_current_place(Inst {
            kind: InstKind::Param(idx),
            ty: ty.clone(),
        });

        self.builder.declare_var(var.id, ty.clone());
        self.builder.write_variable_cur(var.id, val).unwrap();

        Ok((val, ty))
    }
}

// This implementation is the main tac-generation part.
//
// I try to use the method in https://pp.ipd.kit.edu/uploads/publikationen/braun13cc.pdf
// to directly generate SSA code from AST.
//
// Notes:
//
// - All basic blocks that are passed from one statement visitor method into another should already
//   have all their predecessors determined. Any statement visitor method could mark the input basic
//   block as filled and sealed.
impl<'a> AstVisitor for FuncCompiler<'a> {
    type LExprResult = Result<(u32, Ty), Error>;

    type ExprResult = Result<(Value, Ty), Error>;

    type TyResult = Result<Ty, Error>;

    type StmtResult = Result<(), Error>;

    type ProgramResult = ();

    type FuncResult = Result<(), Error>;

    fn visit_func(&mut self, func: &FuncStmt) -> Self::FuncResult {
        self.scope_builder.borrow_mut().add_scope();
        let initial = self.builder.new_bb();
        self.builder.set_current_bb(initial);
        self.builder.func.bb_set_first(initial);
        self.builder.mark_sealed(initial);

        let return_ty = self.visit_ty(&func.ret_ty)?;
        let mut params_ty = vec![];
        for (idx, param) in func.params.iter().enumerate() {
            let (_param_op, param_ty) = self.visit_func_param_real(param, idx)?;
            params_ty.push(param_ty);
        }
        let func_ty = Ty::func_of(return_ty, params_ty);
        self.builder.set_type(func_ty.clone());

        let func_name = &func.name.name;
        self.scope_builder
            .borrow_mut()
            .insert_global(func_name, func_ty);

        self.visit_block_stmt(&func.body)?;

        self.builder.mark_filled(self.builder.current_bb_id());

        self.scope_builder.borrow_mut().pop_scope().unwrap();
        Ok(())
    }

    fn visit_ty(&mut self, _ty: &TyDef) -> Self::TyResult {
        match _ty.name.as_str() {
            "void" => Ok(Ty::Unit),
            "int" => Ok(Ty::int()),
            _ => Err(Error::UnknownType(_ty.name.clone())),
        }
    }

    fn visit_literal_expr(&mut self, _expr: &LiteralExpr) -> Self::ExprResult {
        match _expr.kind {
            LiteralKind::Integer(val) => Ok((Value::Imm(val as i64), Ty::int())),
            LiteralKind::Float(_) => {
                todo!("implement float (or not)")
            }
            LiteralKind::String(_) => {
                todo!("Implement String")
            }
            LiteralKind::Char(ch) => Ok((Value::Imm(ch as i64), Ty::int())),
        }
    }

    fn visit_ident_expr(&mut self, expr: &Ident) -> Self::ExprResult {
        let scope = self.scope_builder.borrow();
        let var = scope
            .find(&expr.name)
            .ok_or_else(|| Error::UnknownVar(expr.name.clone()))?;
        let val = self.builder.read_variable_cur(var.id).unwrap();
        Ok((val.into(), var.ty.clone()))
    }

    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::ExprResult {
        let (var_id, var_ty) = self.visit_lexpr(&expr.lhs)?;
        let (val, val_ty) = self.visit_expr(&expr.rhs)?;

        assert_type_eq(&var_ty, &val_ty)?;

        let result_idx = match val {
            Value::Dest(i) => {
                self.builder.write_variable_cur(var_id, i).unwrap();
                i
            }
            Value::Imm(i) => {
                let target = self.builder.insert_after_current_place(Inst {
                    kind: InstKind::Assign(Value::Imm(i)),
                    ty: val_ty,
                });
                self.builder.write_variable_cur(var_id, target).unwrap();
                target
            }
        };
        Ok((result_idx.into(), Ty::unit()))
    }

    fn visit_lexpr(&mut self, expr: &Expr) -> Self::LExprResult {
        let expr = match expr {
            Expr::Ident(i) => i,
            _ => return Err(Error::InvalidLExpr(format!("{:?}", &expr))),
        };
        let scope = self.scope_builder.borrow();
        let var = scope
            .find(&expr.name)
            .ok_or_else(|| Error::UnknownVar(expr.name.clone()))?;
        Ok((var.id, var.ty.clone()))
    }

    fn visit_binary_expr(&mut self, expr: &BinaryExpr) -> Self::ExprResult {
        let (lhsv, lhst) = self.visit_expr(&expr.lhs)?;
        let (rhsv, rhst) = self.visit_expr(&expr.rhs)?;

        assert_type_eq(&lhst, &rhst)?;

        let (op, ty) = match expr.op {
            BinaryOp::Add => (tac::BinaryOp::Add, lhst.clone()),
            BinaryOp::Sub => (tac::BinaryOp::Sub, lhst.clone()),
            BinaryOp::Mul => (tac::BinaryOp::Mul, lhst.clone()),
            BinaryOp::Div => (tac::BinaryOp::Div, lhst.clone()),
            BinaryOp::Gt => (tac::BinaryOp::Gt, Ty::bool()),
            BinaryOp::Lt => (tac::BinaryOp::Lt, Ty::bool()),
            BinaryOp::Ge => (tac::BinaryOp::Ge, Ty::bool()),
            BinaryOp::Le => (tac::BinaryOp::Le, Ty::bool()),
            BinaryOp::Eq => (tac::BinaryOp::Eq, Ty::bool()),
            BinaryOp::Neq => (tac::BinaryOp::Ne, Ty::bool()),
        };

        let v = self.builder.insert_after_current_place(Inst {
            kind: InstKind::Binary(BinaryInst {
                op,
                lhs: lhsv,
                rhs: rhsv,
            }),
            ty,
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
            UnaryOp::Deref => {
                let ty = t
                    .as_ptr()
                    .ok_or_else(|| Error::UnknownType(format!("{} is not a pointer", t).into()))?;
                let ty = (&**ty).clone();
                let v = v
                    .get_inst()
                    .ok_or_else(|| Error::UnknownVar(format!("{} is not a variable", v).into()))?;
                let v = self.builder.insert_after_current_place(Inst {
                    kind: InstKind::Load(v),
                    ty: ty.clone(),
                });
                Ok((v.into(), ty))
            }
            UnaryOp::Ref => {
                todo!()
            }
        }
    }

    fn visit_call_expr(&mut self, expr: &CallExpr) -> Self::ExprResult {
        let func_ty = self
            .scope_builder
            .borrow()
            .find(&expr.func.name)
            .ok_or_else(|| Error::UnknownVar(expr.func.name.clone()))?
            .ty
            .clone();

        let func_ty = func_ty.as_func().unwrap();

        let mut params = vec![];
        let mut types = vec![];
        for subexpr in &expr.params {
            let (val, ty) = self.visit_expr(&subexpr)?;
            params.push(val);
            types.push(ty);
        }

        if types.len() != func_ty.params.len() {
            return Err(Error::WrongParamLength {
                expected: func_ty.params.len(),
                found: types.len(),
            });
        }
        for (ty, expected) in types.iter().zip(func_ty.params.iter()) {
            assert_type_eq(ty, expected)?;
        }

        let val = self.builder.insert_after_current_place(Inst {
            kind: InstKind::FunctionCall(FunctionCall {
                name: self.interner.borrow_mut().intern(&expr.func.name),
                params,
            }),
            ty: func_ty.return_type.clone(),
        });

        Ok((val.into(), func_ty.return_type.clone()))
    }

    fn visit_as_expr(&mut self, expr: &AsExpr) -> Self::ExprResult {
        self.visit_expr(&expr.val)
    }

    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::StmtResult {
        self.scope_builder.borrow_mut().add_scope();
        for substmt in &stmt.stmts {
            self.visit_stmt(substmt)?;
        }
        self.scope_builder.borrow_mut().pop_scope().unwrap();
        Ok(())
    }

    fn visit_while_stmt(&mut self, stmt: &WhileStmt) -> Self::StmtResult {
        let cur_bb = self.builder.current_bb_id();
        let cond_bb = self.builder.new_bb();
        self.builder.add_branch(cur_bb, cond_bb);
        self.builder.func.bb_get_mut(cur_bb).branch = Branch::Jump(cond_bb);

        self.builder.mark_filled(cur_bb);

        self.builder.set_current_bb(cond_bb);
        self.builder.func.bb_set_after(cur_bb, cond_bb);

        let (cond, _cond_ty) = self.visit_expr(&stmt.cond)?;

        let loop_bb = self.builder.new_bb();
        self.builder.func.bb_set_after(loop_bb, cond_bb);
        let next_bb = self.builder.new_bb();

        self.break_targets.push(BreakTarget {
            break_out: next_bb,
            continue_in: cond_bb,
        });

        self.builder.mark_filled(cond_bb);

        // cond_bb --> loop_bb
        //   \---> next_bb
        self.builder.func.bb_get_mut(cond_bb).branch = Branch::CondJump {
            cond,
            if_true: loop_bb,
            if_false: next_bb,
        };
        self.builder.add_branch(cond_bb, loop_bb);
        self.builder.add_branch(cond_bb, next_bb);
        self.builder.mark_sealed(loop_bb);

        self.builder.set_current_bb(loop_bb);
        self.visit_block_stmt(&stmt.body)?;
        let loop_end_bb = self.builder.current_bb_id();

        self.builder.func.bb_get_mut(loop_end_bb).branch = Branch::Jump(cond_bb);
        self.builder.add_branch(loop_end_bb, cond_bb);

        self.builder.mark_filled(loop_end_bb);
        self.builder.mark_sealed(cond_bb);

        self.break_targets.pop();

        self.builder.func.bb_set_after(loop_end_bb, next_bb);
        self.builder.set_current_bb(next_bb);
        self.builder.mark_sealed(next_bb);

        Ok(())
    }

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::StmtResult {
        let expr_val = self.visit_expr(&stmt.cond)?;
        let last_bb = self.builder.current_bb_id();

        self.builder.mark_filled(last_bb);

        // Create if block
        let if_bb = self.builder.new_bb();
        self.builder.func.bb_set_after(last_bb, if_bb);

        // if -> if_bb
        self.builder.add_branch(last_bb, if_bb);
        self.builder.mark_sealed(if_bb);

        self.builder.set_current_bb(if_bb);
        self.visit_block_stmt(&stmt.if_block)?;

        let if_end_bb = self.builder.current_bb_id();

        // next_bb: The basic block after the if statement
        // Deal with else block
        let next_bb = match &stmt.else_block {
            other @ IfElseBlock::Block(..) | other @ IfElseBlock::If(..) => {
                let else_bb = self.builder.new_bb();
                self.builder.func.bb_set_after(if_end_bb, else_bb);

                // if
                //  \--> else_bb
                self.builder.add_branch(last_bb, else_bb);
                self.builder.mark_sealed(else_bb);
                self.builder.func.bb_get_mut(last_bb).branch = Branch::CondJump {
                    cond: expr_val.0,
                    if_true: if_bb,
                    if_false: else_bb,
                };

                self.builder.set_current_bb(else_bb);

                match other {
                    IfElseBlock::None => unreachable!(),
                    IfElseBlock::If(i) => self.visit_if_stmt(&i)?,
                    IfElseBlock::Block(b) => self.visit_block_stmt(&b)?,
                }
                let else_end_bb = self.builder.current_bb_id();

                let next_bb = self.builder.new_bb();

                self.builder.func.bb_get_mut(else_end_bb).branch = Branch::Jump(next_bb);
                self.builder.add_branch(else_end_bb, next_bb);

                self.builder.mark_filled(else_end_bb);
                self.builder.func.bb_set_after(else_end_bb, next_bb);
                next_bb
            }
            azuki_syntax::ast::IfElseBlock::None => {
                let next_bb = self.builder.new_bb();
                self.builder.func.bb_set_after(if_end_bb, next_bb);

                // if
                //  \--> next_bb
                self.builder.add_branch(last_bb, next_bb);
                self.builder.func.bb_get_mut(last_bb).branch = Branch::CondJump {
                    cond: expr_val.0,
                    if_true: if_bb,
                    if_false: next_bb,
                };
                next_bb
            }
        };

        // if_end_bb -> next_bb
        self.builder.func.bb_get_mut(if_end_bb).branch = Branch::Jump(next_bb);
        self.builder.add_branch(if_end_bb, next_bb);
        self.builder.mark_filled(if_end_bb);

        self.builder.mark_sealed(next_bb);
        self.builder.set_current_bb(next_bb);
        Ok(())
    }

    fn visit_expr_stmt(&mut self, stmt: &Expr) -> Self::StmtResult {
        self.visit_expr(stmt)?;
        Ok(())
    }

    fn visit_decl_stmt(&mut self, stmt: &DeclStmt) -> Self::StmtResult {
        let ty = self.visit_ty(&stmt.ty)?;
        let var_id = self
            .scope_builder
            .borrow_mut()
            .insert(&stmt.name.name, ty.clone())
            .ok_or_else(|| Error::DuplicateVar(stmt.name.name.clone()))?
            .id;
        self.builder.declare_var(var_id, ty);

        if let Some(expr) = &stmt.val {
            let (inst, _) = self.visit_assign_expr(&AssignExpr {
                span: stmt.span,
                allow_assign_const: stmt.is_const,
                lhs: Rc::new(Expr::Ident(Ident {
                    span: stmt.span,
                    name: stmt.name.name.clone(),
                })),
                rhs: expr.clone(),
            })?;
            self.builder
                .write_variable_cur(var_id, inst.get_inst().unwrap())
                .unwrap();
        }

        Ok(())
    }

    fn visit_return_stmt(&mut self, stmt: &ReturnStmt) -> Self::StmtResult {
        let val = if let Some(val) = &stmt.val {
            Some(self.visit_expr(&val)?)
        } else {
            None
        };

        let curr_bb = self.builder.current_bb_id();
        self.builder.func.bb_get_mut(curr_bb).branch = Branch::Return(val.map(|x| x.0));

        self.builder.mark_filled(self.builder.current_bb_id());

        let next_bb = self.builder.new_bb();
        self.builder.set_current_bb(next_bb);
        self.builder.mark_sealed(next_bb);
        self.builder.func.bb_set_after(curr_bb, next_bb);

        Ok(())
    }

    fn visit_break_stmt(&mut self, _span: azuki_syntax::span::Span) -> Self::StmtResult {
        let break_target = self.break_targets.last().unwrap().break_out;

        let cur_bb = self.builder.current_bb_id();
        self.builder.func.bb_get_mut(cur_bb).branch = Branch::Jump(break_target);
        self.builder.add_branch(cur_bb, break_target);

        self.builder.mark_filled(cur_bb);

        let next_bb = self.builder.new_bb();
        self.builder.set_current_bb(next_bb);
        self.builder.mark_sealed(next_bb);
        self.builder.func.bb_set_after(cur_bb, next_bb);

        Ok(())
    }

    fn visit_continue_stmt(&mut self, _span: azuki_syntax::span::Span) -> Self::StmtResult {
        let continue_target = self.break_targets.last().unwrap().continue_in;

        let cur_bb = self.builder.current_bb_id();
        self.builder.func.bb_get_mut(cur_bb).branch = Branch::Jump(continue_target);
        self.builder.add_branch(cur_bb, continue_target);

        self.builder.mark_filled(cur_bb);

        let next_bb = self.builder.new_bb();
        self.builder.set_current_bb(next_bb);
        self.builder.mark_sealed(next_bb);
        self.builder.func.bb_set_after(cur_bb, next_bb);

        Ok(())
    }

    fn visit_empty_stmt(&mut self, _span: azuki_syntax::span::Span) -> Self::StmtResult {
        Ok(())
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
