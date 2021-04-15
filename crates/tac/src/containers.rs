use std::fmt::Display;

use slotmap::{new_key_type, Key, KeyData};

macro_rules! setup_index {
    ($ty:ty) => {
        impl $ty {
            pub fn slot(self) -> u32 {
                slot_num(self.data().as_ffi())
            }

            pub fn from_bits(v: u64) -> Self {
                Self(KeyData::from_ffi(v))
            }

            pub fn into_bits(self) -> u64 {
                self.data().as_ffi()
            }
        }

        impl From<$ty> for u32 {
            fn from(val: $ty) -> Self {
                val.slot()
            }
        }
    };
}

new_key_type! {
    /// The index of a basic block.
    pub struct BBId;

    /// The index of an instruction.
    pub struct InstId;
}

fn slot_num(slotmap_index: u64) -> u32 {
    // extract the lower 32 bits
    slotmap_index as u32
}

setup_index!(BBId);
setup_index!(InstId);

impl Display for BBId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.slot())
    }
}

impl Display for InstId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.slot())
    }
}
