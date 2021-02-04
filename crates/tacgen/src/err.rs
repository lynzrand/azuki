use azuki_tac::Ty;
use smol_str::SmolStr;

#[derive(Debug)]
pub enum Error {
    UnknownType(SmolStr),
    DuplicateVar(SmolStr),
    UnknownVar(SmolStr),
    InvalidLExpr(String),
    WrongParamLength { expected: usize, found: usize },
    TypeMismatch { expected: Ty, found: Ty },
}
