//! Error and Result types.

use generational_arena::Index;

#[derive(Debug)]
pub enum Error {
    NoSuchTacIdx(Index),
    NoSuchBB(usize),
    NoSuchVar(usize),
    NoCodeAfterInst(Index),
    AlreadyConnected,
    NotConnected,
}

pub type TacResult<T> = Result<T, Error>;
