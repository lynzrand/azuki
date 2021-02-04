#![cfg(test)]
use std::{cell::RefCell, rc::Rc};

use azuki_syntax::{
    ast::{BlockStmt, FuncStmt, Ident, TyDef},
    lexer::spanned_lexer,
    parser,
    visitor::AstVisitor,
};
use azuki_tac::builder::FuncBuilder;

use crate::{
    symbol::{NumberingCounter, ScopeBuilder, StringInterner},
    FuncCompiler,
};

#[test]
fn test_basic_func_generation() {
    let program = r"
    fn fib(n: int) -> int {
        if n <= 1 {
            return 1;
        } else {
            return fib(n - 1) + fib(n - 2);
        }
    }
    ";
    let mut parser = parser::Parser::new(spanned_lexer(program));
    let program = parser.parse().unwrap();
    let func = &program.funcs[0];

    let interner = Rc::new(RefCell::new(StringInterner::new()));
    let counter = Rc::new(NumberingCounter::new(0));

    let mut compiler = FuncCompiler {
        builder: FuncBuilder::new("test".into()),
        break_targets: vec![],
        interner: interner.clone(),
        scope_builder: Rc::new(RefCell::new(ScopeBuilder::new(counter, interner))),
    };

    compiler.visit_func(&func).unwrap();

    let func = compiler.builder.build();
    eprintln!("{}", func);
}
