use generational_arena::Index;

pub enum Error {
    NoSuchTacIdx(Index),
    NoCodeAfterInst(Index),
    AlreadyConnected,
    NotConnected,
}

pub type TacResult<T> = Result<T, Error>;
