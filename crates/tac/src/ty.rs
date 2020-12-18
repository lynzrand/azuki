//! Type system definitions and stuff.

pub struct Ty {
    pub kind: TyKind,
    pub size: usize,
}

impl Ty {
    pub fn int() -> Ty {
        Ty {
            kind: TyKind::Int,
            size: 32,
        }
    }

    pub fn bool() -> Ty {
        Ty {
            kind: TyKind::Bool,
            size: 32,
        }
    }
}

pub enum TyKind {
    Bool,
    Int,
}
