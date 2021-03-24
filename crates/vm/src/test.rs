#![cfg(test)]
use crate::Vm;

use azuki_tac::parser::parse_program_from_string;

#[test]
fn run_fib() {
    let input = r"
    (fn fib (i32) i32
        (bb0 (
            (%0 i32 param 0)
            (%1 b32 le %0 1))
            (brif %1 bb1 bb2))
        (bb1 (
            (%7 i32 1))
            (br bb3))
        (bb2 (
            (%2 i32 sub %0 1)
            (%3 i32 sub %0 2)
            (%4 i32 call fib (%2))
            (%5 i32 call fib (%3))
            (%6 i32 add %4 %5))
            (br bb3))
        (bb3 (
            (%8 i32 phi (bb1 %7) (bb2 %6)))
            (return %8)))

    (fn main () i32 
        (bb0 (
            (%0 i32 call fib (15)))
            (return %0)))
    ";
    let result = parse_program_from_string(input).unwrap();
    let mut vm = Vm::new(&result);
    let run_fib = vm.run_func("main", vec![]);
    assert_eq!(run_fib, Some(987));
}

#[test]
fn run_a_plus_b() {
    let input = r"
    (fn add (i32 i32) i32
        (bb0 (
            (%0 i32 param 0)
            (%1 i32 param 1)
            (%2 i32 add %0 %1))
            (return %2)))
    ";
    let result = parse_program_from_string(input).unwrap();

    let mut vm = Vm::new(&result);
    let run_fib = vm.run_func("add", vec![1, 2]);
    assert_eq!(run_fib, Some(3));
}
