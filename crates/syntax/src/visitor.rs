use std::default::Default;

use crate::{ast::*, span::Span};

fn default<T: Default>() -> T {
    T::default()
}

macro_rules! walk {
    ($name:ident ($v:ident , $tgt:ident : $ty:ty) $(-> $r:ty )? $body:block) => {
        pub fn $name<V: AstVisitor + ?Sized>($v: &mut V, $tgt: & $ty) $(-> $r)? {
            $body
        }
    };
}

/// A handy trait for traversing the abstract syntax tree.
///
/// Every function inside this visitor should call the corresponding
/// `walk_<type>` function in order to traverse deeper into the tree.
pub trait AstVisitor {
    type LexprResult: Default;
    type ExprResult: Default;
    type TyResult: Default;
    type StmtResult: Default;
    type ProgramResult: Default;
    type FuncResult: Default;

    fn visit_program(&mut self, program: &Program) -> Self::ProgramResult {
        for decl in &program.decls {
            self.visit_decl_stmt(decl);
        }
        for func in &program.funcs {
            self.visit_func(func);
        }
        default()
    }

    fn visit_func(&mut self, func: &FuncStmt) -> Self::FuncResult {
        for param in &func.params {
            self.visit_func_param(param);
        }
        self.visit_block_stmt(&func.body);
        default()
    }

    fn visit_func_param(&mut self, param: &FuncParam) -> Self::StmtResult {
        default()
    }

    fn visit_ty(&mut self, ty: &TyDef) -> Self::TyResult {
        default()
    }

    fn visit_expr(&mut self, expr: &Expr) -> Self::ExprResult {
        walk_expr(self, expr)
    }

    fn visit_literal_expr(&mut self, expr: &LiteralExpr) -> Self::ExprResult {
        default()
    }

    fn visit_ident_expr(&mut self, expr: &Ident) -> Self::ExprResult {
        default()
    }

    fn visit_assign_expr(&mut self, expr: &AssignExpr) -> Self::ExprResult {
        self.visit_lexpr(&expr.lhs);
        self.visit_expr(&expr.rhs);
        default()
    }

    fn visit_lexpr(&mut self, expr: &Expr) -> Self::LexprResult {
        default()
    }

    fn visit_binary_expr(&mut self, expr: &BinaryExpr) -> Self::ExprResult {
        self.visit_expr(&expr.lhs);
        self.visit_expr(&expr.rhs);
        default()
    }

    fn visit_unary_expr(&mut self, expr: &UnaryExpr) -> Self::ExprResult {
        self.visit_expr(&expr.expr);
        default()
    }

    fn visit_call_expr(&mut self, expr: &CallExpr) -> Self::ExprResult {
        for subexpr in &sexpr.params {
            self.visit_expr(&subexpr);
        }
        default()
    }

    fn visit_as_expr(&mut self, expr: &AsExpr) -> Self::ExprResult {
        self.visit_ty(&expr.ty);
        self.visit_expr(&expr.val);
        default()
    }

    fn visit_stmt(&mut self, stmt: &Stmt) -> Self::StmtResult {
        walk_stmt(self, stmt)
    }

    fn visit_block_stmt(&mut self, stmt: &BlockStmt) -> Self::StmtResult {
        for substmt in &stmt.stmts {
            self.visit_stmt(substmt);
        }
        default()
    }

    fn visit_while_stmt(&mut self, stmt: &WhileStmt) -> Self::StmtResult {
        self.visit_expr(&stmt.cond);
        self.visit_block_stmt(&stmt.body);
        default()
    }

    fn visit_if_stmt(&mut self, stmt: &IfStmt) -> Self::StmtResult {
        self.visit_expr(&stmt.cond);
        self.visit_block_stmt(&stmt.if_block);
        match &stmt.else_block {
            IfElseBlock::None => {}
            IfElseBlock::If(stmt) => {
                self.visit_if_stmt(&stmt);
            }
            IfElseBlock::Block(blk) => {
                self.visit_block_stmt(&blk);
            }
        }
        default()
    }

    fn visit_expr_stmt(&mut self, stmt: &Expr) -> Self::StmtResult {
        self.visit_expr(stmt);
        default()
    }

    fn visit_decl_stmt(&mut self, stmt: &DeclStmt) -> Self::StmtResult {
        self.visit_ty(stmt.ty);
        if let Some(expr) = &stmt.val {
            self.visit_expr(expr);
        }
        default()
    }

    fn visit_return_stmt(&mut self, stmt: &ReturnStmt) -> Self::StmtResult {
        if let Some(res) = stmt.val {
            self.visit_expr(&res);
        }
        default()
    }

    fn visit_break_stmt(&mut self, span: Span) -> Self::StmtResult {
        default()
    }

    fn visit_continue_stmt(&mut self, span: Span) -> Self::StmtResult {
        default()
    }

    fn visit_empty_stmt(&mut self, span: Span) -> Self::StmtResult {
        default()
    }
}

walk! { walk_stmt(v, stmt: Stmt) -> V::StmtResult {
    match stmt {
        Stmt::Block(b) => v.visit_block_stmt(b),
        Stmt::While(s) => {v.visit_while_stmt(s)}
        Stmt::If(s) => {v.visit_if_stmt(s)}
        Stmt::Expr(s) => {v.visit_expr_stmt(s)}
        Stmt::Decl(s) => {v.visit_decl_stmt(s)}
        Stmt::Return(s) => {v.visit_return_stmt(s)}
        Stmt::Break(s) => {v.visit_break_stmt(*s)}
        Stmt::Continue(s) => {v.visit_continue_stmt(*s)}
        Stmt::Empty(s) => {v.visit_empty_stmt(*s)}
    }
}}

walk! { walk_expr(v, expr: Expr) -> V::ExprResult{
    match expr{
        Expr::As(x)=>{v.visit_as_expr(x)}
        Expr::Binary(b)=>{v.visit_binary_expr(b)}
        Expr::Assign(x)=>{v.visit_assign_expr(x)}
        Expr::Call(x)=>{v.visit_call_expr(x)}
        Expr::Ident(x)=>{v.visit_ident_expr(x)}
        Expr::Literal(x)=>{v.visit_literal_expr(x)}
        Expr::Unary(x)=>{v.visit_unary_expr(x)}
    }
}}
