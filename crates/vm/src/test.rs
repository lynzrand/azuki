use crate::Vm;
use azuki_syntax::parse;
use azuki_tacgen::compile;

#[test]
fn run_fib() {
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
    let result = compile(&program).unwrap();

    let mut vm = Vm::new(&result);
    let run_fib = vm.run_func("fib", vec![5]);
    assert_eq!(run_fib, Some(8));
}

#[test]
fn run_a_plus_b() {
    let input = r"
    fn add(a: int, b:int) -> int {
        return a+b;
    }
    ";
    let program = parse(input).unwrap();
    let result = compile(&program).unwrap();

    let mut vm = Vm::new(&result);
    let run_fib = vm.run_func("add", vec![1,2]);
    assert_eq!(run_fib, Some(3));
}
