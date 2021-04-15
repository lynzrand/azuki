//! Type system definitions and stuff.
use enum_as_inner::EnumAsInner;
use std::sync::Arc;

mod registry;

pub const PTR_SIZE: usize = 8;

/// A representation of basic type that has O(1) clone and sizes no more than
/// `2 * sizeof(usize)`.
///
/// > I know this is worse than using an external type repository, but hey you
/// > can directly compare these!
#[derive(Debug, Clone, PartialEq, Eq, EnumAsInner, Hash)]
pub enum Ty {
    Unit,
    Func(Arc<FuncTy>),
    Ptr(Arc<Ty>),
    Numeric(NumericTy),
}

impl Ty {
    pub fn int() -> Ty {
        Ty::Numeric(NumericTy::int())
    }

    pub fn bool() -> Ty {
        Ty::Numeric(NumericTy::bool())
    }

    pub fn unit() -> Ty {
        Ty::Unit
    }

    pub fn func_of(return_type: Ty, params: Vec<Ty>) -> Ty {
        Ty::Func(Arc::new(FuncTy {
            return_type,
            params,
        }))
    }

    pub fn ptr_of(ty: Ty) -> Ty {
        Ty::Ptr(Arc::new(ty))
    }

    pub fn size(&self) -> Option<usize> {
        match self {
            Ty::Unit => Some(0),
            Ty::Func(_) => None,
            Ty::Ptr(_) => Some(PTR_SIZE),
            Ty::Numeric(n) => Some(n.size() as usize),
        }
    }
}

impl Default for Ty {
    fn default() -> Self {
        Ty::Unit
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NumericTy {
    pub kind: TyKind,
    pub size: u8,
}

impl NumericTy {
    pub fn int() -> NumericTy {
        NumericTy {
            kind: TyKind::Int,
            size: 32,
        }
    }

    pub fn bool() -> NumericTy {
        NumericTy {
            kind: TyKind::Bool,
            size: 32,
        }
    }

    pub fn size(&self) -> u8 {
        self.size
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TyKind {
    Bool,
    Int,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncTy {
    pub return_type: Ty,
    pub params: Vec<Ty>,
}

// pub struct TypeInterner {
//     interned: HashSet<Arc<Ty>>,
// }

// impl TypeInterner {
//     /// Intern the given type so it takes up less space.
//     pub fn intern_ty(&mut self, ty: Ty) -> Ty {
//         match &ty {
//             Ty::Unit => ty,
//             Ty::Numeric(_) => ty,
//             Ty::Func(f) => {}
//             Ty::Ptr(p) => {}
//         }
//     }

//     pub fn interned_arced(&mut self, ty: Arc<Ty>) -> Arc<Ty> {
//         if self.interned
//         match ty{
//             Ty::Unit => {}
//             Ty::Numeric(_) => {}
//             Ty::Func(_) => {}
//             Ty::Ptr(_) => {}
//         }
//     }
// }
