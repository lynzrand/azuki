use azuki_tac::Ty;

#[derive(Debug)]
pub enum Error {
    TypeMismatch { expected: Ty, found: Ty },
}
