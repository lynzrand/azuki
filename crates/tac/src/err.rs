//! Error and Result types.

use thunderdome::Index;

use crate::BBId;

#[derive(Debug)]
pub enum Error {
    NoSuchTacIdx(Index),
    NoSuchBB(BBId),
    NoSuchVar(String),
    NoCodeAfterInst(Index),
    AlreadyConnected,
    NotConnected,
}

pub type TacResult<T> = Result<T, Error>;
