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

    pub fn expect_span(reason: Cow<'static, str>, span: lexpr::datum::Span) -> Self {
        Self::spanned(Expect(reason), span)
    }

    pub fn expect_pos(reason: Cow<'static, str>, pos: lexpr::parse::Position) -> Self {
        Self::position(Expect(reason), pos)
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
        ParseError::expect_span(
            format!("a valid number width, got {}", e).into(),
            val.span(),
        )
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
        .ok_or_else(|| ParseError::expect_span("a basic block ID number".into(), val.span()))?
        .map_err(|e| {
            ParseError::expect_span(
                format!("basic block ID number, got {}", e).into(),
                val.span(),
            )
        })?;
    Ok(ctx.declared_bb(val))
}

fn parse_inst_id(val: LRef, ctx: &mut VariableNamingCtx) -> Result<InstId, ParseError> {
    let id = val
        .as_name()
        .and_then(|x| x.starts_with('%').then(|| usize::from_str(&x[1..])))
        .ok_or_else(|| ParseError::expect_span("instruction ID number".into(), val.span()))?
        .map_err(|e| {
            ParseError::expect_span(
                format!("instruction ID number, got {}", e).into(),
                val.span(),
            )
        })?;

    Ok(ctx.declared_var(id))
}

fn parse_inst_kind(
    mut val_iter: ListIter,
    ctx: &mut VariableNamingCtx,
) -> Result<InstKind, ParseError> {
    todo!()
}

// (<dest> <ty> <opcode> ...<params>)
fn parse_inst(val: LRef<'_>, ctx: &mut VariableNamingCtx) -> Result<(), ParseError> {
    let mut inst_iter = val.list_iter().ok_or_else(|| {
        ParseError::expect_span("An instruction represented as a list".into(), val.span())
    })?;

    let inst_id = inst_iter
        .next()
        .ok_or_else(|| ParseError::expect_pos("instruction ID number".into(), val.span().end()))?;
    let inst_id = parse_inst_id(inst_id, ctx)?;

    let ty = inst_iter
        .next()
        .ok_or_else(|| ParseError::expect_pos("instruction value type".into(), val.span().end()))?;
    let ty = parse_type(ty)?;

    let kind = parse_inst_kind(inst_iter, ctx)?;

    let inst = Inst { kind, ty };
    *ctx.func.func.inst_get_mut(inst_id) = inst;
    Ok(())
}

// (<bb_id> <instruction_list> <branch>)
fn parse_bb(val: LRef<'_>, ctx: &mut VariableNamingCtx) -> Result<(), ParseError> {
    let mut list = val
        .list_iter()
        .ok_or_else(|| ParseError::spanned(Expect("Basic block definition".into()), val.span()))?;

    let id = list.next().ok_or_else(|| {
        ParseError::expect_pos("the ID of a basic block".into(), val.span().end())
    })?;
    let id = parse_bb_id(id, ctx)?;

    // parse instruction list
    let instructions = list
        .next()
        .and_then(|x| x.list_iter())
        .ok_or_else(|| ParseError::expect_pos("a list of instructions".into(), val.span().end()))?;
    instructions
        .map(|x| parse_inst(x, ctx))
        .collect::<Result<Vec<_>, _>>()?;

    todo!()
}

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
