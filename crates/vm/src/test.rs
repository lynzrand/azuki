use crate::Vm;
use azuki_syntax::parse;
use azuki_tac::parser::{self, parse_func, parse_program, parse_program_from_string, EasyParser};
use azuki_tacgen::compile;

#[test]
fn run_fib() {
    let input = r"
    fn @fib(i32) -> i32 {
    bb0:
        %0 = i32 param 0
        %1 = i32 le %0 #1
        br bb1 if %1
        br bb2
    bb1:
        return #1
    bb2:
        %2 = i32 sub %0 #1
        %3 = i32 sub %0 #2
        %4 = i32 call @fib(%2)
        %5 = i32 call @fib(%3)
        %6 = i32 add %4 %5
        return %6
    }
    ";
    let result = parse_program_from_string(input).unwrap();
    let mut vm = Vm::new(&result);
    let run_fib = vm.run_func("fib", vec![5]);
    assert_eq!(run_fib, Some(8));
}

#[test]
fn run_a_plus_b() {
    let input = r"
    fn @add(i32, i32) -> i32 {
    bb0:
        %0 = i32 param 0
        %1 = i32 param 1
        %2 = i32 add %0 %1
        return %2
    }
    ";
    let result = parse_program_from_string(input).unwrap();

    let mut vm = Vm::new(&result);
    let run_fib = vm.run_func("add", vec![1, 2]);
    assert_eq!(run_fib, Some(3));
}
