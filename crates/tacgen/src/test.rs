#![cfg(test)]
use std::{cell::RefCell, rc::Rc};

use azuki_syntax::{lexer::spanned_lexer, parser, visitor::AstVisitor};
use azuki_tac::TacFunc;

use crate::{
    symbol::{NumberingCounter, ScopeBuilder, StringInterner},
    FuncCompiler,
};

#[test]
fn test_basic_func_generation() {
    let program = r"
    fn fib(n: int) -> int {
        let r: int;
        while 0 != 1 {
            r = r;
        }
        if n <= 1 {
            r = 1;
        } else {
            r = fib(n - 1) + fib(n - 2);
        }
        return r;
    }
    ";
    let mut parser = parser::Parser::new(spanned_lexer(program));
    let program = parser.parse().unwrap();
    let func = &program.funcs[0];

    let interner = Rc::new(RefCell::new(StringInterner::new()));
    let counter = Rc::new(NumberingCounter::new(0));

    let scope_builder = Rc::new(RefCell::new(ScopeBuilder::new(counter, interner.clone())));
    let mut result = TacFunc::new_untyped("fib".into());
    let mut compiler = FuncCompiler::new(&mut result, interner, scope_builder);

    compiler.visit_func(&func).unwrap();
    compiler.builder.sanity_check();

    eprintln!("{}", result);
}
