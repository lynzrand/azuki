//! Serialization and de-serialization for TAC code.

use indexmap::IndexSet;
use std::{cell::Cell, fmt::Display};
use ty::FuncTy;

use crate::*;

trait FormatContext<C> {
    fn fmt_ctx(&self, f: &mut std::fmt::Formatter<'_>, ctx: C) -> std::fmt::Result;
}

impl<T, C> FormatContext<C> for &T
where
    T: FormatContext<C>,
{
    fn fmt_ctx(&self, f: &mut std::fmt::Formatter<'_>, ctx: C) -> std::fmt::Result {
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

impl Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Unit => {
                write!(f, "unit")
            }
            Ty::Func(func) => func.fmt(f),
            Ty::Ptr(tgt) => {
                write!(f, "{}*", tgt)
            }
            Ty::Numeric(ty) => ty.fmt(f),
        }
    }
}

impl Display for FuncTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn(")?;
        for (idx, param) in self.params.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }
            param.fmt(f)?;
        }
        write!(f, ") -> {}", &self.return_type)
    }
}

impl Display for NumericTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            TyKind::Bool => {
                write!(f, "b")
            }
            TyKind::Int => {
                write!(f, "i")
            }
        }?;
        write!(f, "{}", self.size)
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

impl FormatContext<&mut TacFormatCtx> for Value {
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

impl FormatContext<(VarId, &mut TacFormatCtx)> for Tac {
    fn fmt_ctx(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        ctx: (VarId, &mut TacFormatCtx),
    ) -> std::fmt::Result {
        write!(f, "{} = ", ctx.0)?;
        write!(f, "<{}> ", self.inst.ty)?;
        match &self.inst.kind {
            InstKind::Binary(i) => {
                write!(f, "{:?} ", i.op)?;
                i.lhs.fmt_ctx(f, ctx.1)?;
                write!(f, " ")?;
                i.rhs.fmt_ctx(f, ctx.1)?;
            }
            InstKind::FunctionCall(call) => {
                write!(f, "call @{} (", &call.name)?;
                for (idx, param) in call.params.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    param.fmt_ctx(f, ctx.1)?;
                }
                write!(f, ")")?;
            }
            InstKind::Const(i) => {
                write!(f, "const {}", i)?;
            } // InstKind::Jump(_) => {}
            // InstKind::CondJump { cond, target } => {}
            InstKind::Param => {
                write!(f, "param")?;
            } // InstKind::Return(v) => {
              //     write!(f, "return ")?;
              //     v.fmt_ctx(f, ctx.1)?;
              // }
        }
        Ok(())
    }
}

impl FormatContext<&mut TacFormatCtx> for Branch {
    fn fmt_ctx(&self, f: &mut std::fmt::Formatter<'_>, ctx: &mut TacFormatCtx) -> std::fmt::Result {
        match self {
            Branch::Return(v) => {
                write!(f, "return ")?;
                if let Some(val) = v {
                    val.fmt_ctx(f, ctx)?;
                }
            }
            Branch::Jump(target) => {
                target.fmt_ctx(f, ctx)?;
            }
            Branch::CondJump { cond, target } => {
                write!(f, "if ")?;
                cond.fmt_ctx(f, ctx)?;
                write!(f, " ")?;
                target.fmt_ctx(f, ctx)?;
            }
            Branch::TableJump { .. } => {
                todo!("No table jump for now");
            }
        }
        Ok(())
    }
}

impl FormatContext<&mut TacFormatCtx> for BranchTarget {
    fn fmt_ctx(&self, f: &mut std::fmt::Formatter<'_>, ctx: &mut TacFormatCtx) -> std::fmt::Result {
        write!(f, "jump {} (", self.bb)?;
        for (idx, (target_idx, source_idx)) in self.params.iter().enumerate() {
            if idx != 0 {
                write!(f, ", ")?;
            }
            let target_var_id = ctx.var_id(*target_idx);
            let source_var_id = ctx.var_id(*source_idx);
            write!(f, "{} <- {}", target_var_id, source_var_id)?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

impl std::fmt::Display for TacFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "fn @{}: {} {{", &self.name, &self.ty)?;
        let mut ctx = TacFormatCtx {
            i_set: IndexSet::new(),
        };
        for (k, v) in &self.basic_blocks {
            writeln!(f, "bb {}:", k)?;
            if let Some(x) = v.head {
                let mut cur_idx = x;
                loop {
                    let i = self.arena.get(cur_idx).unwrap();
                    let cur_id = ctx.var_id(cur_idx);
                    write!(f, "\t")?;
                    i.fmt_ctx(f, (cur_id, &mut ctx))?;
                    writeln!(f)?;
                    match i.next {
                        Some(x) => cur_idx = x,
                        None => {
                            break;
                        }
                    }
                }
            }
            for target in &v.jumps {
                write!(f, "\t")?;
                target.fmt_ctx(f, &mut ctx)?;
                writeln!(f)?;
            }
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}
