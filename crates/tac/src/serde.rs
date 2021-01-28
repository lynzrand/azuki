use indexmap::IndexSet;
use std::{cell::Cell, fmt::Display};

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
        match &self.inst.kind {
            InstKind::Binary(i) => {
                write!(f, "{:?} ", i.op)?;
                i.lhs.fmt_ctx(f, ctx.1)?;
                write!(f, " ")?;
                i.rhs.fmt_ctx(f, ctx.1)?;
            }
            InstKind::FunctionCall(_) => {}
            InstKind::Const(i) => {
                write!(f, "const {:?} {}", self.inst.ty, i)?;
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

impl std::fmt::Display for TacFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "fn {} {{", &self.name)?;
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
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}
