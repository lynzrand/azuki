//! Serialization and de-serialization for TAC code.

use indexmap::IndexSet;
use std::{fmt::Display, writeln};
use ty::FuncTy;
use util::ListFormatter;

use crate::*;

pub trait FormatContext<C> {
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
        // VarId(self.i_set.insert_full(var).0)
        VarId(var.slot())
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

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "sub",
            BinaryOp::Mul => "mul",
            BinaryOp::Div => "div",
            BinaryOp::Lt => "lt",
            BinaryOp::Gt => "gt",
            BinaryOp::Le => "le",
            BinaryOp::Ge => "ge",
            BinaryOp::Eq => "eq",
            BinaryOp::Ne => "ne",
        };
        write!(f, "{}", s)
    }
}

struct VarId(u32);
impl Display for VarId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 != u32::max_value() {
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
        write!(f, "{} ", self.inst.ty)?;
        match &self.inst.kind {
            InstKind::Binary(i) => {
                write!(f, "{} ", i.op)?;
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

            InstKind::Assign(i) => {
                i.fmt_ctx(f, ctx.1)?;
            }
            InstKind::Param(id) => {
                write!(f, "param {}", id)?;
            }
            InstKind::Phi(phi) => {
                write!(f, "phi [")?;
                let mut first = true;
                for (&bb, &val) in phi {
                    if !first {
                        write!(f, ", ")?;
                    } else {
                        first = false;
                    }
                    write!(f, "({}, bb{})", ctx.1.var_id(val), bb.unique_num())?;
                }
                write!(f, "]")?;
            }
            InstKind::Dead => {
                write!(f, "dead_value")?;
            }
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
                write!(f, "br bb{}", target.unique_num())?;
            }
            Branch::CondJump { cond, target } => {
                write!(f, "br bb{} if ", target.unique_num())?;
                cond.fmt_ctx(f, ctx)?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for TacFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = self.ty.as_func().unwrap();
        let param_fmt = ListFormatter::new(ty.params.iter());
        writeln!(
            f,
            "fn @{}({}) -> {} {{",
            &self.name, param_fmt, &ty.return_type
        )?;
        let mut ctx = TacFormatCtx {
            i_set: IndexSet::new(),
        };

        // let mut reverse_dfs_path =
        //     BiasedRevPostOrderDfs::new(&self.basic_blocks, self.starting_block);
        // while let Some(k) = reverse_dfs_path.next(&self.basic_blocks) {
        for (k, v) in self.bb_iter() {
            writeln!(f, "bb{}:", k.unique_num())?;
            if let Some(x) = v.head {
                let mut cur_idx = x;
                loop {
                    let i = self.instructions_arena.get(cur_idx).unwrap();
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
            if v.jumps.is_empty() {
                writeln!(f, "\tunreachable")?;
            }
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}
