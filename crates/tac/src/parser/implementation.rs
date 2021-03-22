#![cfg(feature = "parser")]
//! Module for parsing the text representation of Azuki TAC into real code.
//!
//! An ANTLR specification of Azuki TAC's text representation can be found in
//! `/docs/src/tac/AzukiTac.g4`.

pub use combine::easy as easy_parse;
pub use combine::stream as parse_stream;
pub use combine::{EasyParser, Parser, Stream};

use combine::{
    attempt, between, choice, eof,
    error::StreamError,
    many, many1, one_of, optional, parser,
    parser::{
        char::{alpha_num, char, digit, hex_digit, newline, spaces as nl_spaces, string},
        combinator::ignore,
    },
    stream::StreamErrorFor,
    ParseError,
};
use smol_str::SmolStr;
use std::{cell::RefCell, collections::BTreeMap, fmt::Display, ops::Neg};

use crate::{
    builder::FuncEditor, BBId, BinaryInst, BinaryOp, Branch, FunctionCall, Inst, InstId, InstKind,
    NumericTy, Program, TacFunc, Ty, TyKind, Value,
};

struct VariableNamingCtx<'f> {
    func: FuncEditor<'f>,
    local_vars: BTreeMap<usize, InstId>,
    bb_id_map: BTreeMap<u32, BBId>,
    last_bb: Option<BBId>,
}

impl<'f> VariableNamingCtx<'f> {
    pub fn new(func: &'f mut TacFunc) -> VariableNamingCtx<'f> {
        VariableNamingCtx {
            func: FuncEditor::new(func),
            local_vars: BTreeMap::new(),
            bb_id_map: BTreeMap::new(),
            last_bb: None,
        }
    }

    pub fn declared_var(&mut self, var_id: usize) -> InstId {
        if let Some(&mapping) = self.local_vars.get(&var_id) {
            mapping
        } else {
            // insert placeholder instruction
            let var = self.func.func.inst_new(Inst {
                kind: InstKind::empty_phi(),
                ty: Ty::unit(),
            });
            self.local_vars.insert(var_id, var);
            var
        }
    }

    pub fn set_var(&mut self, idx: InstId, inst: Inst) {
        let inst_ref = self.func.func.tac_get_mut(idx);
        inst_ref.inst = inst;
    }

    pub fn declared_bb(&mut self, bb_id: u32) -> BBId {
        let func = &mut self.func;
        *self.bb_id_map.entry(bb_id).or_insert_with(|| func.new_bb())
    }
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
    (
        spaces0(),
        choice((
            string("\r\n").map(|_| ()),
            char('\n').map(|_| ()),
            char('\r').map(|_| ()),
        )),
        nl_spaces(),
    )
        .map(|_| ())
}

/// Parse a comma-separated list. The internal parser should skip spaces.
fn comma_sep_list<TOut, TList, I, P>(parse_internal: P) -> impl Parser<I, Output = TList>
where
    P: Parser<I, Output = TOut>,
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    TList: Default + Extend<TOut>,
{
    combine::sep_by(parse_internal, char(',').skip(spaces0()))
}

fn ident<Input>() -> impl Parser<Input, Output = String>
where
    Input: Stream<Token = char>,
{
    (char('@'), many1(alpha_num())).map(|x| x.1)
}

fn unsigned_dec_number<I, N>() -> impl Parser<I, Output = N>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    N: num::Num,
    N::FromStrRadixErr: Display,
{
    many1(digit()).and_then(|digits: String| {
        N::from_str_radix(&digits, 10).map_err(StreamErrorFor::<I>::message_format)
    })
}

fn dec_number<I, N>() -> impl Parser<I, Output = N>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    N: num::Num + Neg<Output = N>,
    N::FromStrRadixErr: Display,
{
    (
        optional(choice((char('-'), char('+')))),
        unsigned_dec_number(),
    )
        .map(|(neg, x): (_, N)| if neg == Some('-') { -x } else { x })
}

fn hex_number<I, N>() -> impl Parser<I, Output = N>
where
    I: Stream<Token = char>,
    I::Error: ParseError<I::Token, I::Range, I::Position>,
    N: num::Num,
    N::FromStrRadixErr: Display,
{
    (string("0x"), many1(hex_digit())).and_then(|(_, digits)| {
        let _: &String = &digits;
        N::from_str_radix(&digits, 16).map_err(StreamErrorFor::<I>::message_format)
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
    (char('%'), unsigned_dec_number::<_, usize>()).map(|(_, digits)| digits)
}

fn value<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = Value> + 'a
where
    Input: Stream<Token = char> + 'a,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    choice((
        number().map(Value::Imm),
        variable().map(move |v| Value::Dest(ctx.borrow_mut().declared_var(v))),
    ))
}

fn bb_id<Input>() -> impl Parser<Input, Output = u32>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (string("bb"), unsigned_dec_number::<_, u32>()).map(|(_, bb_id)| bb_id)
}
// ========= Types ==========

fn int_ty<Input>() -> impl Parser<Input, Output = Ty>
where
    Input: Stream<Token = char>,
{
    (char('i'), unsigned_dec_number::<_, u8>()).and_then(|(_, size)| {
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
    (char('b'), unsigned_dec_number::<_, u8>()).and_then(|(_, size)| {
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
        between(
            char('(').skip(spaces0()),
            char(')').skip(spaces0()),
            comma_sep_list(ty().skip(spaces0())).skip(spaces0()),
        ),
        string("->").skip(spaces0()),
        ty(),
    )
        .map(|(_, params, _, ret_ty)| Ty::func_of(ret_ty, params))
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
        attempt(string("add")),
        attempt(string("sub")),
        attempt(string("mul")),
        attempt(string("div")),
        attempt(string("gt")),
        attempt(string("lt")),
        attempt(string("ge")),
        attempt(string("le")),
        attempt(string("eq")),
        attempt(string("ne")),
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

fn binary_instruction<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = BinaryInst> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    (
        binary_op().skip(spaces1()),
        value(ctx).skip(spaces1()),
        value(ctx),
    )
        .map(|(op, lhs, rhs)| BinaryInst { op, lhs, rhs })
}

fn value_instruction<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = Value> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    value(ctx)
}

fn func_call_instruction<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = FunctionCall> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    (
        string("call").skip(spaces1()),
        ident().skip(spaces0()),
        between(
            string("(").skip(spaces0()),
            string(")"),
            comma_sep_list(value(ctx)).skip(spaces0()),
        ),
    )
        .map(|(_, func, params)| FunctionCall {
            name: func.into(),
            params,
        })
}

fn phi_instruction<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = BTreeMap<BBId, InstId>> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    (
        string("phi").skip(spaces0()),
        between(
            char('[').skip(spaces0()),
            char(']').skip(spaces0()),
            comma_sep_list(between(
                char('(').skip(spaces0()),
                char(')').skip(spaces0()),
                (
                    variable().skip(spaces0()),
                    char(',').skip(spaces0()),
                    bb_id().skip(spaces0()),
                )
                    .map(move |(var, _, bb)| {
                        let mut ctx = ctx.borrow_mut();
                        let val = ctx.declared_var(var);
                        let bb = ctx.declared_bb(bb);
                        (bb, val)
                    }),
            )),
        ),
    )
        .map(|(_, list)| list)
}

fn param_instruction<'a, Input>() -> impl Parser<Input, Output = usize> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    (string("param").skip(spaces1()), unsigned_dec_number()).map(|(_, i)| i)
}

fn instruction<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = ()> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    (
        variable().skip(spaces0()).skip(string("=")).skip(spaces0()),
        ty().skip(spaces1()),
        choice((
            attempt(param_instruction().map(InstKind::Param)),
            attempt(binary_instruction(ctx).map(InstKind::Binary)),
            attempt(value_instruction(ctx).map(InstKind::Assign)),
            attempt(func_call_instruction(ctx).map(InstKind::FunctionCall)),
            attempt(phi_instruction(ctx).map(InstKind::Phi)),
        )),
    )
        .map(move |(v, ty, kind)| {
            let inst = Inst { kind, ty };
            let mut ctx = ctx.borrow_mut();
            let idx = ctx.declared_var(v);
            ctx.set_var(idx, inst);
            ctx.func.put_inst_after_current_place(idx);
        })
}

fn branch_if_jump_instruction<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = Branch> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    (
        string("if").skip(spaces1()),
        value(ctx),
        string("br").skip(spaces1()),
        bb_id(),
        string("else").skip(spaces1()),
        bb_id(),
    )
        .map(move |(_, cond, _, if_true, _, if_false)| {
            let mut ctx = ctx.borrow_mut();

            let if_true = ctx.declared_bb(if_true);
            let if_false = ctx.declared_bb(if_false);

            Branch::CondJump {
                cond,
                if_true,
                if_false,
            }
        })
}

fn branch_jump_instruction<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = Branch> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    (string("br").skip(spaces1()), bb_id()).map(move |(_, target)| {
        let mut ctx = ctx.borrow_mut();
        let target = ctx.declared_bb(target);
        Branch::Jump(target)
    })
}

fn unreachable_jump_instruction<Input>() -> impl Parser<Input, Output = Branch>
where
    Input: Stream<Token = char>,
{
    string("unreachable").map(|_| Branch::Unreachable)
}

fn return_jump_instruction<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = Branch> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    (string("return"), optional(attempt((spaces1(), value(ctx))))).map(move |(_, val)| {
        let mut ctx = ctx.borrow_mut();
        let val = val.map(|(_, v)| v);
        Branch::Return(val)
    })
}

fn jump_instructions<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = Branch> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    choice((
        attempt(unreachable_jump_instruction().skip(nl1())),
        attempt(return_jump_instruction(ctx).skip(nl1())),
        attempt(branch_jump_instruction(ctx).skip(nl1())),
        attempt(branch_if_jump_instruction(ctx).skip(nl1())),
    ))
}

fn single_basic_block<'a, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
) -> impl Parser<Input, Output = ()> + 'a
where
    Input: Stream<Token = char> + 'a,
{
    // all parsers commit to the result
    (bb_id().skip(spaces0()), string(":").skip(nl1()))
        .message("When parsing BB label")
        .then(move |(id, _)| {
            let bb_id;
            {
                let mut ctx = ctx.borrow_mut();
                bb_id = ctx.declared_bb(id);
                // ctx.func.func.bb_seq.push(bb_id);
                ctx.func.set_current_bb(bb_id);
                match ctx.last_bb {
                    Some(last_bb) => {
                        ctx.func.func.bb_set_after(last_bb, bb_id);
                    }
                    None => {
                        ctx.func.func.bb_set_first(bb_id);
                    }
                }
                ctx.last_bb = Some(bb_id);
            }
            many(attempt(
                instruction(ctx)
                    .message("When parsing instruction")
                    .skip(nl1()),
            ))
            .map(|_: ()| ())
            .and(attempt(
                jump_instructions(ctx)
                    .message("When parsing jump instructions")
                    .map(move |branch| {
                        ctx.borrow_mut().func.func.bb_get_mut(bb_id).branch = branch;
                    }),
            ))
        })
        .map(|_| ())
}

fn basic_blocks<'b, Input>(
    ctx: &'b RefCell<VariableNamingCtx<'b>>,
) -> impl Parser<Input, Output = ()> + 'b
where
    Input: Stream<Token = char> + 'b,
{
    // this parser edits the internal states of `ctx`, thus returns `()`
    many1(attempt(single_basic_block(ctx)))
}

fn func_header<I>() -> impl Parser<I, Output = (SmolStr, Ty)>
where
    I: Stream<Token = char>,
{
    (
        string("fn").skip(spaces1()),
        ident().skip(spaces0()),
        between(
            string("(").skip(spaces0()),
            string(")").skip(spaces0()),
            comma_sep_list(ty().skip(spaces0())).skip(spaces0()),
        ),
        string("->").skip(spaces0()),
        ty(),
    )
        .message("When parsing function header")
        .map(|(_, name, params, _, ret_ty)| (name.into(), Ty::func_of(ret_ty, params)))
}

pub fn parse_func<'a, I>() -> impl Parser<I, Output = TacFunc>
where
    I: Stream<Token = char> + 'a,
{
    combine::parser::function::parser(|i| {
        let mut func = TacFunc::default();
        let ctx = RefCell::new(VariableNamingCtx::new(&mut func));
        let res = (
            func_header().skip(spaces0()),
            between(
                string("{").skip(nl1()),
                string("}").skip(nl1()),
                basic_blocks(&ctx),
            ),
        )
            .message("When parsing function")
            .parse_stream(i);
        res.map(|((name, ty), _)| {
            func.name = name;
            func.ty = ty;
            func
        })
        .into_result()
    })
}

pub fn parse_program<'a, I>() -> impl Parser<I, Output = Program>
where
    I: Stream<Token = char> + 'a,
{
    ignore(nl_spaces())
        .and(many(parse_func().map(|f| (f.name.clone(), f))))
        .skip(nl_spaces())
        .skip(eof())
        .map(|(_, functions)| Program { functions })
}
