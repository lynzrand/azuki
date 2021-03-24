#![cfg(feature = "parser")]
//! Module for parsing the text representation of Azuki TAC into real code.
//!
//! An ANTLR specification of Azuki TAC's text representation can be found in
//! `/docs/src/tac/AzukiTac.g4`.

use std::{
    borrow::{Borrow, Cow},
    collections::BTreeMap,
    fs::Permissions,
    str::FromStr,
};

use crate::{
    builder::FuncEditor, BBId, BasicBlock, BinaryInst, BinaryOp, Branch, FunctionCall, Inst,
    InstId, InstKind, NumericTy, Program, Tac, TacFunc, Ty, TyKind, Value,
};

use lexpr::{
    datum::Ref as LRef,
    datum::{ListIter, Span},
    Datum, Value as LexprVal,
};
use ParseErrorKind::*;

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

#[derive(Debug)]
pub enum ParseErrorKind {
    ExpectFunctionDef,
    ExpectName(Cow<'static, str>),
    Expect(Cow<'static, str>),
}

#[derive(Debug)]
pub enum Position {
    None,
    Span(lexpr::datum::Span),
    Position(lexpr::parse::Position),
}

#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    at: Position,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind) -> ParseError {
        ParseError {
            kind,
            at: Position::None,
        }
    }

    pub fn spanned(kind: ParseErrorKind, span: lexpr::datum::Span) -> ParseError {
        ParseError {
            kind,
            at: Position::Span(span),
        }
    }

    pub fn position(kind: ParseErrorKind, pos: lexpr::parse::Position) -> ParseError {
        ParseError {
            kind,
            at: Position::Position(pos),
        }
    }

    pub fn expect_span(reason: impl Into<Cow<'static, str>>, span: lexpr::datum::Span) -> Self {
        Self::spanned(Expect(reason.into()), span)
    }

    pub fn expect_pos(reason: impl Into<Cow<'static, str>>, pos: lexpr::parse::Position) -> Self {
        Self::position(Expect(reason.into()), pos)
    }

    pub fn expect(reason: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Expect(reason.into()))
    }
}

trait WithPosition {
    fn with_position(self, at: Position) -> Self;
    fn with_position_if_none(self, at: Position) -> Self;
}

impl WithPosition for ParseError {
    fn with_position(mut self, at: Position) -> Self {
        self.at = at;
        self
    }

    fn with_position_if_none(mut self, at: Position) -> Self {
        if let Position::None = self.at {
            self.at = at
        }
        self
    }
}
impl<T> WithPosition for Result<T, ParseError> {
    fn with_position(self, at: Position) -> Self {
        self.map_err(|e| e.with_position(at))
    }

    fn with_position_if_none(self, at: Position) -> Self {
        self.map_err(|e| e.with_position_if_none(at))
    }
}

fn expect_name(val: LRef<'_>, name: Cow<'static, str>) -> Result<(), ParseError> {
    if val.value().as_name().map_or(false, |x| x == name) {
        Ok(())
    } else {
        Err(ParseError::spanned(
            ParseErrorKind::ExpectName(name),
            val.span(),
        ))
    }
}

fn expect_opt_name(
    val: Option<LRef<'_>>,
    name: Cow<'static, str>,
    parent_pos: lexpr::parse::Position,
) -> Result<(), ParseError> {
    match val {
        Some(v) => expect_name(v, name),
        None => Err(ParseError::position(
            ParseErrorKind::ExpectName(name),
            parent_pos,
        )),
    }
}

fn parsed_named_type(val: LRef<'_>) -> Result<Ty, ParseError> {
    let name = val
        .as_name()
        .ok_or_else(|| ParseError::spanned(Expect("Type name".into()), val.span()))?;
    let ty_kind = match name.chars().next().unwrap() {
        'i' => TyKind::Int,
        'b' => TyKind::Bool,
        _ => {
            return Err(ParseError::spanned(
                Expect("Type starting with 'i' or 'b'".into()),
                val.span(),
            ))
        }
    };
    let len = &name[1..];
    let size = u8::from_str(len).map_err(|e| {
        ParseError::expect_span(format!("a valid number width, got {}", e), val.span())
    })?;
    Ok(Ty::Numeric(NumericTy {
        kind: ty_kind,
        size,
    }))
}

fn parse_type(val: LRef<'_>) -> Result<Ty, ParseError> {
    if val.is_null() {
        Ok(Ty::unit())
    } else {
        parsed_named_type(val)
    }
}

fn parse_bb_id(val: LRef<'_>, ctx: &mut VariableNamingCtx) -> Result<BBId, ParseError> {
    let val = val
        .as_name()
        .and_then(|n| n.starts_with("bb").then(|| u32::from_str(&n[2..])))
        .ok_or_else(|| ParseError::expect_span("a basic block ID number", val.span()))?
        .map_err(|e| {
            ParseError::expect_span(format!("basic block ID number, got {}", e), val.span())
        })?;
    Ok(ctx.declared_bb(val))
}

fn parse_inst_id(val: LRef, ctx: &mut VariableNamingCtx) -> Result<InstId, ParseError> {
    let id = val
        .as_name()
        .and_then(|x| x.starts_with('%').then(|| usize::from_str(&x[1..])))
        .ok_or_else(|| ParseError::expect_span("instruction ID number", val.span()))?
        .map_err(|e| {
            ParseError::expect_span(format!("instruction ID number, got {}", e), val.span())
        })?;

    Ok(ctx.declared_var(id))
}

fn parse_value(val: LRef<'_>, ctx: &mut VariableNamingCtx) -> Result<Value, ParseError> {
    if let Some(v) = val.as_i64() {
        Ok(Value::Imm(v))
    } else {
        parse_inst_id(val, ctx).map(|x| x.into())
    }
}

fn parse_binary_inst_rest(
    mut val_iter: ListIter,
    ctx: &mut VariableNamingCtx,
    op: BinaryOp,
) -> Result<InstKind, ParseError> {
    let lhs = val_iter
        .next()
        .ok_or_else(|| ParseError::expect("left hand side value of binary op"))?;
    let lhs = parse_value(lhs, ctx)?;

    let rhs = val_iter
        .next()
        .ok_or_else(|| ParseError::expect("right hand side value of binary op"))?;
    let rhs = parse_value(rhs, ctx)?;

    Ok(InstKind::Binary(BinaryInst { op, lhs, rhs }))
}

fn parse_param_rest(mut val_iter: ListIter) -> Result<InstKind, ParseError> {
    let next = val_iter
        .next()
        .ok_or_else(|| ParseError::expect("an integer"))?;
    let next = next
        .as_u64()
        .ok_or_else(|| ParseError::expect_span("an integer", next.span()))?;
    Ok(InstKind::Param(next as usize))
}

fn parse_call_rest(
    mut val_iter: ListIter,
    ctx: &mut VariableNamingCtx,
) -> Result<InstKind, ParseError> {
    let name = val_iter
        .next()
        .ok_or_else(|| ParseError::expect("Function name"))?;
    let name = name.as_name().ok_or_else(|| {
        ParseError::expect("Function name should be a symbol or string")
            .with_position(Position::Span(name.span()))
    })?;

    let param_list = val_iter
        .next()
        .ok_or_else(|| ParseError::expect("Function param list"))?;
    let param_list = param_list.list_iter().ok_or_else(|| {
        ParseError::expect_span("Function param list should be a list", param_list.span())
    })?;

    let params = param_list
        .map(|x| parse_value(x, ctx))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(InstKind::FunctionCall(FunctionCall {
        name: name.into(),
        params,
    }))
}

fn parse_phi_rest(val_iter: ListIter, ctx: &mut VariableNamingCtx) -> Result<InstKind, ParseError> {
    let phi = val_iter
        .map(|x| {
            let (bb, id) = x
                .as_pair()
                .ok_or_else(|| ParseError::expect_span("phi item", x.span()))?;
            let bb = parse_bb_id(bb, ctx)?;
            let id = parse_inst_id(id, ctx)?;
            Ok((bb, id))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    Ok(InstKind::Phi(phi))
}

fn parse_inst_kind(
    mut val_iter: ListIter,
    ctx: &mut VariableNamingCtx,
) -> Result<InstKind, ParseError> {
    let name = val_iter
        .next()
        .ok_or_else(|| ParseError::expect("instruction name or immediate"))?;
    let name_span = name.span();

    // Early return - number case
    if let Some(n) = name.as_i64() {
        return Ok(InstKind::Assign(Value::Imm(n)));
    }

    let name = name
        .as_name()
        .ok_or_else(|| ParseError::expect("instruction name"))?;

    match name {
        "add" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Add),
        "sub" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Sub),
        "mul" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Mul),
        "div" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Div),
        "eq" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Eq),
        "ne" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Ne),
        "lt" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Lt),
        "gt" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Gt),
        "le" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Le),
        "ge" => parse_binary_inst_rest(val_iter, ctx, BinaryOp::Ge),

        "param" => parse_param_rest(val_iter),

        "call" => parse_call_rest(val_iter, ctx),

        "phi" => parse_phi_rest(val_iter, ctx),

        n if n.starts_with('%') => {
            let id = usize::from_str(n)
                .map_err(|e| ParseError::expect_span(format!("var id, got {}", e), name_span))?;
            let id = ctx.declared_var(id);
            Ok(InstKind::Assign(Value::Dest(id)))
        }

        _ => Err(ParseError::expect_span(
            "A valid instruction name",
            name_span,
        )),
    }
}

// (<dest> <ty> <opcode> ...<params>)
fn parse_inst(val: LRef<'_>, ctx: &mut VariableNamingCtx) -> Result<(), ParseError> {
    let mut inst_iter = val.list_iter().ok_or_else(|| {
        ParseError::expect_span("An instruction represented as a list", val.span())
    })?;

    let inst_id = inst_iter
        .next()
        .ok_or_else(|| ParseError::expect_pos("instruction ID number", val.span().end()))?;
    let inst_id = parse_inst_id(inst_id, ctx)?;

    let ty = inst_iter
        .next()
        .ok_or_else(|| ParseError::expect_pos("instruction value type", val.span().end()))?;
    let ty = parse_type(ty)?;

    let kind = parse_inst_kind(inst_iter, ctx)
        .with_position_if_none(Position::Position(val.span().end()))?;

    let inst = Inst { kind, ty };
    *ctx.func.func.inst_get_mut(inst_id) = inst;
    ctx.func.put_inst_after_current_place(inst_id);

    Ok(())
}

fn parse_branch(val: LRef, ctx: &mut VariableNamingCtx) -> Result<Branch, ParseError> {
    let mut iter = val
        .list_iter()
        .ok_or_else(|| ParseError::expect_span("Branch instruction", val.span()))?;

    let name = iter
        .next()
        .ok_or_else(|| ParseError::expect_pos("branch instruction name", val.span().end()))?;
    let name_span = name.span();
    let name = name
        .as_name()
        .ok_or_else(|| ParseError::expect_span("branch instruction name", name.span()))?;

    Ok(match name {
        "br" => {
            let target = parse_bb_id(
                iter.next().ok_or_else(|| {
                    ParseError::expect_pos("target basic block", val.span().end())
                })?,
                ctx,
            )?;
            Branch::Jump(target)
        }
        "brif" => {
            let cond = parse_value(
                iter.next()
                    .ok_or_else(|| ParseError::expect_pos("condition", val.span().end()))?,
                ctx,
            )?;

            let bb_true = parse_bb_id(
                iter.next().ok_or_else(|| {
                    ParseError::expect_pos("target basic block", val.span().end())
                })?,
                ctx,
            )?;
            let bb_false = parse_bb_id(
                iter.next().ok_or_else(|| {
                    ParseError::expect_pos("target basic block", val.span().end())
                })?,
                ctx,
            )?;
            Branch::CondJump {
                cond,
                if_true: bb_true,
                if_false: bb_false,
            }
        }
        "return" => {
            let val = iter.next();
            let val = val.map(|x| parse_value(x, ctx)).transpose()?;
            Branch::Return(val)
        }
        "unreachable" => Branch::Unreachable,
        _ => {
            return Err(ParseError::expect_span(
                "br, brif, return or unreachable",
                name_span,
            ))
        }
    })
}

// (<bb_id> <instruction_list> <branch>)
fn parse_bb(val: LRef<'_>, ctx: &mut VariableNamingCtx) -> Result<(), ParseError> {
    let mut list = val
        .list_iter()
        .ok_or_else(|| ParseError::expect_span("Basic block definition", val.span()))?;

    let id = list
        .next()
        .ok_or_else(|| ParseError::expect_pos("the ID of a basic block", val.span().end()))?;
    let id = parse_bb_id(id, ctx)?;
    ctx.func.set_current_bb(id);

    // parse instruction list
    let instructions = list
        .next()
        .and_then(|x| x.list_iter())
        .ok_or_else(|| ParseError::expect_pos("a list of instructions", val.span().end()))?;
    instructions
        .map(|x| parse_inst(x, ctx))
        .collect::<Result<Vec<_>, _>>()?;

    let branch = parse_branch(
        list.next()
            .ok_or_else(|| ParseError::expect_pos("branch instruction", val.span().end()))?,
        ctx,
    )?;

    ctx.func.current_bb_mut().branch = branch;
    Ok(())
}

// (fn <name> <param> <return> ...<basic-blocks>)
pub fn parse_function(val: LRef<'_>) -> Result<TacFunc, ParseError> {
    let mut list = val
        .list_iter()
        .ok_or_else(|| ParseError::spanned(ExpectFunctionDef, val.span()))?;

    expect_opt_name(list.next(), "fn".into(), val.span().end())?;

    let name = list.next().unwrap();
    let name = name.as_name().unwrap();

    let params = list.next().and_then(|x| x.list_iter()).ok_or_else(|| {
        ParseError::position(
            ParseErrorKind::Expect("'()' or list of params".into()),
            val.span().end(),
        )
    })?;
    let params = params.map(parse_type).collect::<Result<Vec<_>, _>>()?;

    let ret = list.next().map(parse_type).ok_or_else(|| {
        ParseError::position(
            ParseErrorKind::Expect("'()' or return type".into()),
            val.span().end(),
        )
    })??;

    let func_ty = Ty::func_of(ret, params);
    let mut func = TacFunc::new(name.into(), func_ty);
    let mut ctx = VariableNamingCtx::new(&mut func);

    list.try_for_each(|x| parse_bb(x, &mut ctx))?;

    Ok(func)
}
