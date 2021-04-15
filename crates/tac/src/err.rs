//! Error and Result types.

use crate::{BBId, InstId};

#[derive(Debug)]
pub enum Error {
    NoSuchTacIdx(InstId),
    NoSuchBb(BBId),
    NoSuchVar(String),
    NoCodeAfterInst(InstId),
    AlreadyConnected,
    NotConnected,
}

pub type TacResult<T> = Result<T, Error>;
