mod lexer;

use combine::{
    between, choice,
    error::StreamError,
    many, many1, one_of, optional, parser,
    parser::{
        char::{alpha_num, char, digit, hex_digit, newline, spaces as nl_spaces, string},
        combinator::factory,
        function::env_parser,
    },
    stream::StreamErrorFor,
    ParseError, Parser, StdParseResult, Stream,
};
use petgraph::Graph;
use smol_str::SmolStr;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    rc::Rc,
    str::FromStr,
    todo,
};

use crate::{
    builder::FuncEditor, util::Captures, BBId, BinaryInst, BinaryOp, FunctionCall, Inst, InstKind,
    NumericTy, OpRef, TacFunc, Ty, TyKind, Value,
};

struct VariableNamingCtx<'f> {
    func: FuncEditor<'f>,
    local_vars: BTreeMap<usize, OpRef>,
    // type_interner: HashSet<Ty>,
    curr_bb: BBId,
}

impl<'f> VariableNamingCtx<'f> {
    pub fn new(func: &'f mut TacFunc) -> VariableNamingCtx<'f> {
        VariableNamingCtx {
            func: FuncEditor::new(func),
            local_vars: BTreeMap::new(),
            // type_interner: HashSet::new(),
            curr_bb: BBId::default(),
        }
    }

    pub fn declared_var(&mut self, var_id: usize) -> OpRef {
        if let Some(&mapping) = self.local_vars.get(&var_id) {
            mapping
        } else {
            // insert placeholder instruction
            self.func.func.tac_new(
                Inst {
                    kind: InstKind::Dead,
                    ty: Ty::unit(),
                },
                self.curr_bb,
            )
        }
    }

    pub fn set_var(&mut self, idx: OpRef, inst: Inst) {
        let inst_ref = self
            .func
            .func
            .arena_get_mut(idx)
            .expect("The supplied index must be valid");
        inst_ref.inst = inst;
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
            binary_instruction(ctx).map(InstKind::Binary),
            value_instruction(ctx).map(InstKind::Assign),
            func_call_instruction(ctx).map(InstKind::FunctionCall),
        )),
    )
        .map(move |(v, ty, kind)| {
            let inst = Inst { kind, ty };
            let mut ctx = ctx.borrow_mut();
            let idx = ctx.declared_var(v);
            ctx.set_var(idx, inst);
            ctx.func.put_inst_after_current_place(idx).unwrap();
        })
}

#[allow(clippy::type_complexity)]
fn single_basic_block<'a: 'b, 'b, Input>(
    ctx: &'a RefCell<VariableNamingCtx<'a>>,
    bb_id_map: &'b RefCell<BTreeMap<i64, BBId>>,
) -> impl Parser<Input, Output = ()> + Captures<'a> + 'b
where
    Input: Stream<Token = char> + 'a,
{
    // all parsers commit to the result
    (
        string("bb"),
        unsigned_dec_number().skip(spaces0()),
        (string(":")),
    )
        .then(move |(_, id, _)| {
            // let (ctx, bb_id_map) = ctx.clone();

            let bb_id = *bb_id_map
                .borrow_mut()
                .entry(id)
                .or_insert_with(|| ctx.borrow_mut().func.new_bb());
            ctx.borrow_mut().func.set_current_bb(bb_id).unwrap();

            many(instruction(ctx)).map(|_: ()| ())
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
    combine::parser::function::parser(move |i| {
        let ctx = &*ctx;
        let bb_id_map = RefCell::new(BTreeMap::new());

        let single_blk = single_basic_block(ctx, &bb_id_map);
        let res = many1(single_blk).parse_stream(i);

        res.into_result()
    })
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
        .map(|(_, name, params, _, ret_ty)| (name.into(), Ty::func_of(ret_ty, params)))
}

pub fn parse_func<'a, I>() -> impl Parser<I, Output = TacFunc>
where
    I: Stream<Token = char> + 'a,
{
    combine::parser::function::parser(|i| {
        let mut func = TacFunc::new_untyped(SmolStr::default());
        let ctx = RefCell::new(VariableNamingCtx::new(&mut func));
        let res = (
            func_header().skip(spaces0()),
            between(
                string("{").skip(nl1()),
                string("}").skip(nl1()),
                basic_blocks(&ctx),
            ),
        )
            .parse_stream(i);
        res.map(|((name, ty), _)| {
            func.name = name;
            func.ty = ty;
            func
        })
        .into_result()
    })
}
