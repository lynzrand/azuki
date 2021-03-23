#![cfg(test)]

use azuki_syntax::parse;
use azuki_tac::parser::EasyParser;
use azuki_tacvm::Vm;

#[test]
fn test_basic_func_generation() {
    let input = r"
    fn fib(n: int) -> int {
        let r: int;
        if n <= 1 {
            r = 1;
        } else {
            r = fib(n - 1) + fib(n - 2);
        }
        return r;
    }
    ";
    let program = parse(input).unwrap();
    let result = crate::compile(&program).unwrap();
    eprintln!("{}", result.functions["fib"]);

    let mut vm = Vm::new(&result);
    assert_eq!(vm.run_func("fib", vec![5]), Some(8));
}
