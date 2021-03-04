#![cfg(test)]

use azuki_syntax::parse;
use azuki_tac::parser::EasyParser;

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

    let res = result.functions["fib"].to_string();

    let stream = azuki_tac::parser::parse_stream::position::Stream::new(res.as_str());
    let parsed = azuki_tac::parser::parse_func().easy_parse(stream);
    match parsed {
        Ok(r) => {
            eprintln!("{}", r.0);
        }
        Err(e) => {
            eprintln!("{}", e);
            panic!("failed");
        }
    }
}
