//! Error and Result types.

use thunderdome::Index;

#[derive(Debug)]
pub enum Error {
    NoSuchTacIdx(Index),
    NoSuchBB(usize),
    NoSuchVar(String),
    NoCodeAfterInst(Index),
    AlreadyConnected,
    NotConnected,
}

pub type TacResult<T> = Result<T, Error>;
