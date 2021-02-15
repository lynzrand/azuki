use combine::{
    choice, easy,
    error::{Format, Info, UnexpectedParse},
    many1, optional,
    parser::char::{alpha_num, char, digit, spaces, string},
    ParseError, Parser, Stream,
};
use std::{str::FromStr, todo};

use crate::{BinaryInst, BinaryOp, InstKind, OpRef, Value};

struct VariableNamingCtx {
    // local_vars:
}

fn ident<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
{
    (char('@'), many1(alpha_num())).map(|x| x.1).skip(spaces())
}

fn num<Input>() -> impl Parser<Input, Output = i64>
where
    Input: Stream<Token = char>,
{
    (char('#'), optional(char('-')), many1(digit()))
        .map(|(_, neg, digits)| {
            let _: &String = &digits;
            let x = i64::from_str(&digits).unwrap();
            if neg.is_some() {
                -x
            } else {
                x
            }
        })
        .skip(spaces())
}

fn variable<Input>() -> impl Parser<Input, Output = usize>
where
    Input: Stream<Token = char>,
{
    (char('%'), many1(digit()))
        .skip(spaces())
        .and_then(|(_, digits): (_, String)| {
            digits.parse::<usize>().unwrap()
            // .map_err(|e| UnexpectedParse::Unexpected.add_expected("unsigned integer"))
        })
}

fn value<Input>() -> impl Parser<Input, Output = Value>
where
    Input: Stream<Token = char>,
{
    choice((
        num().map(Value::Imm),
        // TODO: map input variable number to arena indices
        // THIS IS ONLY A PLACEHOLDER
        variable().map(|v| Value::Dest(OpRef::from_bits(v as u64))),
    ))
}

fn binary_op<Input>() -> impl Parser<Input, Output = BinaryOp>
where
    Input: Stream<Token = char>,
{
    choice([
        string("add"),
        string("sub"),
        string("mul"),
        string("div"),
        string("gt"),
        string("lt"),
        string("ge"),
        string("le"),
        string("eq"),
        string("ne"),
    ])
    .skip(spaces())
    .map(|i| match i {
        "add" => BinaryOp::Add,
        "sub" => BinaryOp::Sub,
        "mul" => BinaryOp::Mul,
        "div" => BinaryOp::Div,
        "gt" => BinaryOp::Gt,
        "lt" => BinaryOp::Lt,
        "ge" => BinaryOp::Ge,
        "le" => BinaryOp::Le,
        "eq" => BinaryOp::Eq,
        "ne" => BinaryOp::Ne,
        _ => unreachable!(),
    })
}

fn binary_instruction<Input>() -> impl Parser<Input, Output = BinaryInst>
where
    Input: Stream<Token = char>,
{
    (binary_op(), value(), value())
        .skip(spaces())
        .map(|(op, lhs, rhs)| BinaryInst { op, lhs, rhs })
}

fn instruction<Input>() -> impl Parser<Input, Output = InstKind>
where
    Input: Stream<Token = char>,
{
    choice((binary_instruction().map(|x| InstKind::Binary(x)),))
}

fn parse_func() {}
