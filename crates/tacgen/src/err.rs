use azuki_tac::Ty;
use smol_str::SmolStr;

#[derive(Debug)]
pub enum Error {
    UnknownType(SmolStr),
    DuplicateVar(SmolStr),
    UnknownVar(SmolStr),
    TypeMismatch { expected: Ty, found: Ty },
}
