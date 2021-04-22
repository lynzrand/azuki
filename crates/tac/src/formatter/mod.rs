//! Serialization and de-serialization for TAC code.

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

impl Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Unit => {
                write!(f, "()")
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
        write!(f, "(fn (")?;
        for (idx, param) in self.params.iter().enumerate() {
            if idx != 0 {
                write!(f, " ")?;
            }
            param.fmt(f)?;
        }
        write!(f, ") {})", &self.return_type)
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

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Dest(i) => {
                write!(f, "{}", i)
            }
            Value::Imm(imm) => {
                write!(f, "{}", imm)
            }
        }
    }
}

impl FormatContext<VarId> for Tac {
    fn fmt_ctx(&self, f: &mut std::fmt::Formatter<'_>, ctx: VarId) -> std::fmt::Result {
        write!(f, "({} {} ", ctx.0, self.inst.ty)?;
        match &self.inst.kind {
            InstKind::Binary(i) => {
                write!(f, "{} {} {}", i.op, i.lhs, i.rhs)?;
            }
            InstKind::FunctionCall(call) => {
                write!(f, "call {} (", &call.name)?;
                for (idx, param) in call.params.iter().enumerate() {
                    if idx != 0 {
                        write!(f, " ")?;
                    }
                    param.fmt(f)?;
                }
                write!(f, ")")?;
            }

            InstKind::Assign(i) => {
                i.fmt(f)?;
            }

            InstKind::Param(id) => {
                write!(f, "param {}", id)?;
            }

            InstKind::Phi(phi) => {
                write!(f, "phi ")?;
                let mut first = true;
                for (&bb, &val) in phi {
                    if !first {
                        write!(f, " ")?;
                    } else {
                        first = false;
                    }
                    write!(f, "({} {})", val, bb)?;
                }
            }

            InstKind::Save(addr, val) => {
                write!(f, "save {} {}", addr, val)?;
            }

            InstKind::Load(addr) => {
                write!(f, "load {}", addr)?;
            }
        }
        write!(f, ")")?;
        Ok(())
    }
}

impl Display for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        match self {
            Branch::Return(v) => {
                write!(f, "return")?;
                if let Some(val) = v {
                    write!(f, " {}", val)?;
                }
            }
            Branch::Jump(target) => {
                write!(f, "br {}", target)?;
            }
            Branch::CondJump {
                cond,
                if_true,
                if_false,
            } => {
                write!(
                    f,
                    "brif {} bb{} bb{}",
                    cond,
                    if_true.slot(),
                    if_false.slot()
                )?;
            }
            Branch::Unreachable => {
                write!(f, "unreachable")?;
            }
        }
        write!(f, ")")?;
        Ok(())
    }
}

impl std::fmt::Display for TacFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = self.ty.as_func().unwrap();
        let param_fmt = ListFormatter::new(ty.params.iter());
        write!(f, "(fn {} ({}) {}", &self.name, param_fmt, &ty.return_type)?;

        for (k, v) in self.bb_iter() {
            writeln!(f)?;
            write!(f, "\t(bb{} (", k.slot())?;
            if let Some(x) = v.head {
                let mut cur_idx = x;
                loop {
                    let i = self.instructions_arena.get(cur_idx).unwrap();
                    let cur_id = cur_idx.slot();

                    writeln!(f)?;
                    write!(f, "\t\t")?;
                    i.fmt_ctx(f, VarId(cur_id))?;

                    match i.next {
                        Some(x) => cur_idx = x,
                        None => {
                            break;
                        }
                    }
                }
            }
            writeln!(f, ")")?;
            write!(f, "\t\t")?;
            v.branch.fmt(f)?;
            write!(f, ")")?;
        }
        writeln!(f, ")")?;
        Ok(())
    }
}
