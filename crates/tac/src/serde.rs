use indexmap::IndexSet;
use std::{cell::Cell, fmt::Display};

use crate::*;

trait FormatContext<C> {
    fn fmt_ctx(&self, f: &mut std::fmt::Formatter<'_>, ctx: &mut C) -> std::fmt::Result;
}

impl<T, C> FormatContext<C> for &T
where
    T: FormatContext<C>,
{
    fn fmt_ctx(&self, f: &mut std::fmt::Formatter<'_>, ctx: &mut C) -> std::fmt::Result {
        (*self).fmt_ctx(f, ctx)
    }
}

struct TacFormatCtx {
    pub i_set: IndexSet<Index>,
}

impl TacFormatCtx {
    pub fn var_id(&mut self, var: Index) -> VarId {
        VarId(self.i_set.insert_full(var).0)
    }
}

struct VarId(usize);
impl Display for VarId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 != usize::max_value() {
            write!(f, "%{}", self.0)
        } else {
            write!(f, "_")
        }
    }
}

impl FormatContext<TacFormatCtx> for Value {
    fn fmt_ctx(&self, f: &mut std::fmt::Formatter<'_>, ctx: &mut TacFormatCtx) -> std::fmt::Result {
        match self {
            Value::Dest(i) => {
                write!(f, "{}", ctx.var_id(*i))
            }
            Value::Imm(imm) => {
                write!(f, "#{}", imm)
            }
        }
    }
}

impl FormatContext<(VarId, TacFormatCtx)> for Tac {
    fn fmt_ctx(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        ctx: &mut (VarId, TacFormatCtx),
    ) -> std::fmt::Result {
        write!(f, "{} = ", ctx.0)?;
        match &self.inst {
            Inst::Binary(i) => {
                write!(f, "{:?} ", i.op)?;
                i.lhs.fmt_ctx(f, &mut ctx.1)?;
                write!(f, " ")?;
                i.rhs.fmt_ctx(f, &mut ctx.1)?;
                writeln!(f)?;
            }
            Inst::FunctionCall(_) => {}
            Inst::Const(_) => {}
            Inst::Jump(_) => {}
            Inst::CondJump { cond, target } => {}
        }
        todo!()
    }
}
