use std::ops::IndexMut as IndexMutOp;
use std::{fmt::Display, ops::Index as IndexOp};

use thunderdome::{Arena, Index};

use crate::BasicBlock;

pub type InstId = thunderdome::Index;

/// The index of a basic block.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BBId(thunderdome::Index);

impl BBId {
    pub fn from_index(idx: thunderdome::Index) -> Self {
        Self(idx)
    }

    pub fn as_index(self) -> thunderdome::Index {
        self.0
    }

    pub fn unique_num(self) -> u32 {
        self.0.slot()
    }
}

impl Display for BBId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.unique_num())
    }
}

impl IndexOp<BBId> for Arena<BasicBlock> {
    type Output = BasicBlock;

    fn index(&self, index: BBId) -> &Self::Output {
        self.index(index.0)
    }
}

impl IndexMutOp<BBId> for Arena<BasicBlock> {
    fn index_mut(&mut self, index: BBId) -> &mut Self::Output {
        self.index_mut(index.0)
    }
}

impl From<Index> for BBId {
    fn from(i: Index) -> Self {
        Self(i)
    }
}

impl Into<Index> for BBId {
    fn into(self) -> Index {
        self.0
    }
}

impl Into<u32> for BBId {
    fn into(self) -> u32 {
        self.0.slot()
    }
}

impl Default for BBId {
    fn default() -> Self {
        // The default value is an unlikely value
        BBId(thunderdome::Index::from_bits(u64::max_value()))
    }
}
