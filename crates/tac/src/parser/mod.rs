use combine::{
    choice, easy,
    error::{Format, Info, StreamError, UnexpectedParse},
    many, many1, one_of, optional,
    parser::char::{alpha_num, char, digit, hex_digit, string},
    stream::StreamErrorFor,
    ParseError, Parser, Stream, StreamOnce,
};
use std::{str::FromStr, todo};

use crate::{err::Error, BinaryInst, BinaryOp, InstKind, OpRef, Value};

struct VariableNamingCtx {
    // local_vars:
}

/// Matches zero or more non-newline space characters
fn spaces<Input>() -> impl Parser<Input, Output = ()>
where
    Input: Stream<Token = char>,
{
    many(one_of(" \t".chars()).map(|_| ()))
}

/// Matches one or more non-newline space characters
fn spaces1<Input>() -> impl Parser<Input, Output = ()>
where
    Input: Stream<Token = char>,
{
    many1(one_of(" \t".chars()).map(|_| ()))
}
/// Parse a comma-separated list. The internal parser should skip spaces.
fn comma_sep_list<TOut, I, P>(parse_internal: P) -> impl Parser<I, Output = Vec<TOut>>
where
    P: Parser<I, Output = TOut>,
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    combine::sep_by(parse_internal, char(',').skip(spaces()))
}

fn ident<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
{
    (char('@'), many1(alpha_num())).map(|x| x.1).skip(spaces())
}

fn dec_number<I>() -> impl Parser<I, Output = i64>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    (optional(choice((char('-'), char('+')))), many1(digit())).and_then(
        |(neg, digits)| -> Result<_, StreamErrorFor<I>> {
            let _: &String = &digits;
            let x = i64::from_str(&digits).map_err(StreamErrorFor::<I>::message_format)?;
            if neg == Some('-') {
                Ok(-x)
            } else {
                Ok(x)
            }
        },
    )
}

fn hex_number<I>() -> impl Parser<I, Output = i64>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    (string("0x"), many1(hex_digit())).and_then(|(_, digits)| {
        let _: &String = &digits;
        i64::from_str_radix(&digits, 16).map_err(StreamErrorFor::<I>::message_format)
    })
}

fn number<Input>() -> impl Parser<Input, Output = i64>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (char('#'), choice((hex_number(), dec_number())))
        .map(|(_, num)| num)
        .skip(spaces())
}

fn variable<Input>() -> impl Parser<Input, Output = usize>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (char('%'), many1(digit()))
        .skip(spaces())
        .and_then(|(_, digits): (_, String)| {
            digits
                .parse::<usize>()
                .map_err(StreamErrorFor::<Input>::message_format)
        })
}

fn value<Input>() -> impl Parser<Input, Output = Value>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    choice((
        number().map(Value::Imm),
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
    .skip(spaces1())
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

fn value_instruction<Input>() -> impl Parser<Input, Output = Value>
where
    Input: Stream<Token = char>,
{
    value()
}

fn instruction<Input>() -> impl Parser<Input, Output = InstKind>
where
    Input: Stream<Token = char>,
{
    choice((
        binary_instruction().map(InstKind::Binary),
        value_instruction().map(InstKind::Assign),
    ))
}

pub fn parse_func() {}
