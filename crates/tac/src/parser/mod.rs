mod lexer;

use combine::{
    choice, easy,
    error::{Format, Info, StreamError, UnexpectedParse},
    many, many1, one_of, optional, parser,
    parser::char::{alpha_num, char, digit, hex_digit, newline, spaces as nl_spaces, string},
    stream::StreamErrorFor,
    ParseError, Parser, Stream, StreamOnce,
};
use petgraph::Graph;
use smol_str::SmolStr;
use std::{str::FromStr, todo};

use crate::{
    err::Error, BinaryInst, BinaryOp, FunctionCall, InstKind, NumericTy, OpRef, TacFunc, Ty,
    TyKind, Value,
};

struct VariableNamingCtx {
    // local_vars:
}

/// Matches zero or more non-newline space characters
fn spaces0<Input>() -> impl Parser<Input, Output = ()>
where
    Input: Stream<Token = char>,
{
    many(one_of(" \t".chars()).map(|_| ()))
}

/// Matches one or more non-newline space characters, or the end of a line
fn spaces1<Input>() -> impl Parser<Input, Output = ()>
where
    Input: Stream<Token = char>,
{
    many1(one_of(" \t".chars()).map(|_| ()))
}

/// Matches some spaces, a new line, and some other spaces or newlines
fn nl1<Input>() -> impl Parser<Input, Output = ()>
where
    Input: Stream<Token = char>,
{
    (spaces0(), newline(), nl_spaces()).map(|_| ())
}

/// Parse a comma-separated list. The internal parser should skip spaces.
fn comma_sep_list<TOut, I, P>(parse_internal: P) -> impl Parser<I, Output = Vec<TOut>>
where
    P: Parser<I, Output = TOut>,
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    combine::sep_by(parse_internal, char(',').skip(spaces0()))
}

fn ident<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
{
    (char('@'), many1(alpha_num())).map(|x| x.1)
}

fn unsigned_dec_number<I>() -> impl Parser<I, Output = i64>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    many1(digit()).and_then(|digits: String| {
        i64::from_str(&digits).map_err(StreamErrorFor::<I>::message_format)
    })
}

fn dec_number<I>() -> impl Parser<I, Output = i64>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
{
    (
        optional(choice((char('-'), char('+')))),
        unsigned_dec_number(),
    )
        .map(|(neg, x)| if neg == Some('-') { -x } else { x })
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
    (char('#'), choice((hex_number(), dec_number()))).map(|(_, num)| num)
}

fn variable<Input>() -> impl Parser<Input, Output = usize>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (char('%'), many1(digit())).and_then(|(_, digits): (_, String)| {
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

// ========= Types ==========

fn int_ty<Input>() -> impl Parser<Input, Output = Ty>
where
    Input: Stream<Token = char>,
{
    (char('i'), dec_number()).and_then(|(_, size)| {
        if size > 64 || (size & size.wrapping_sub(1) != 0) {
            return Err(StreamErrorFor::<Input>::message_format(format_args!(
                "size {} must be smaller than 64 and is a power of 2",
                size
            )));
        }
        Ok(Ty::Numeric(NumericTy {
            kind: TyKind::Int,
            size: size as u8,
        }))
    })
}

fn bool_ty<Input>() -> impl Parser<Input, Output = Ty>
where
    Input: Stream<Token = char>,
{
    (char('b'), dec_number()).and_then(|(_, size)| {
        if size > 64 || (size & size.wrapping_sub(1) != 0) {
            return Err(StreamErrorFor::<Input>::message_format(format_args!(
                "size {} must be smaller than 64 and is a power of 2",
                size
            )));
        }
        Ok(Ty::Numeric(NumericTy {
            kind: TyKind::Bool,
            size: size as u8,
        }))
    })
}

fn unit_ty<Input>() -> impl Parser<Input, Output = Ty>
where
    Input: Stream<Token = char>,
{
    string("unit").map(|_| Ty::unit())
}

fn func_ty<Input>() -> impl Parser<Input, Output = Ty>
where
    Input: Stream<Token = char>,
{
    (
        string("fn").skip(spaces0()),
        char('(').skip(spaces0()),
        comma_sep_list(ty().skip(spaces0())).skip(spaces0()),
        string("->").skip(spaces0()),
        ty(),
    )
        .map(|(_, _, params, _, ret_ty)| Ty::func_of(ret_ty, params))
}

fn _ty<Input>() -> impl Parser<Input, Output = Ty>
where
    Input: Stream<Token = char>,
{
    choice((int_ty(), bool_ty(), unit_ty(), func_ty()))
}

parser! {
    fn ty [Input]()(Input) -> Ty
    where [Input:Stream<Token=char>] {
        _ty()
    }
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
    (
        binary_op().skip(spaces1()),
        value().skip(spaces1()),
        value(),
    )
        .map(|(op, lhs, rhs)| BinaryInst { op, lhs, rhs })
}

fn value_instruction<Input>() -> impl Parser<Input, Output = Value>
where
    Input: Stream<Token = char>,
{
    value()
}

fn func_call_instruction<Input>() -> impl Parser<Input, Output = FunctionCall>
where
    Input: Stream<Token = char>,
{
    (
        string("call").skip(spaces1()),
        ident().skip(spaces0()),
        string("(").skip(spaces0()),
        comma_sep_list(value()).skip(spaces0()),
        string(")"),
    )
        .map(|(_, func, _, params, _)| FunctionCall {
            name: func.into(),
            params,
        })
}

fn instruction<Input>() -> impl Parser<Input, Output = InstKind>
where
    Input: Stream<Token = char>,
{
    choice((
        binary_instruction().map(InstKind::Binary),
        value_instruction().map(InstKind::Assign),
        func_call_instruction().map(InstKind::FunctionCall),
    ))
    .skip(nl1())
}

fn basic_blocks<I>() -> impl Parser<I, Output = Graph<crate::BasicBlock, ()>>
where
    I: Stream<Token = char>,
{
    spaces0().map(|_| todo!())
}

fn func_header<I>() -> impl Parser<I, Output = (SmolStr, Ty)>
where
    I: Stream<Token = char>,
{
    (
        string("fn").skip(spaces1()),
        ident().skip(spaces0()),
        comma_sep_list(ty().skip(spaces0())).skip(spaces0()),
        string("->").skip(spaces0()),
        ty(),
    )
        .map(|(_, name, params, _, ret_ty)| (name.into(), Ty::func_of(ret_ty, params)))
}

pub fn parse_func<I>() -> impl Parser<I, Output = TacFunc>
where
    I: Stream<Token = char>,
{
    (
        func_header().skip(spaces0()),
        string("{").skip(nl1()),
        basic_blocks(),
        string("}").skip(nl1()),
    )
        .map(|((name, ty), _, blocks, _)| TacFunc {
            name,
            ty,
            param_map: todo!(),
            arena: todo!(),
            basic_blocks: blocks,
            starting_block: todo!(),
        })
}
