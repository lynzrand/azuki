use azuki_tac::InstId;

#[derive(Clone, Copy, Debug)]
pub struct StackPointer {
    pub frame: u32,
    pub offset: u32,
    pub value: InstId,
}

#[derive(Clone, Copy, Debug)]
pub enum Value {
    Int(i64),
    Bool(bool),
    StackPointer(StackPointer),
    HeapPointer {},
}

impl From<StackPointer> for Value {
    fn from(v: StackPointer) -> Self {
        Self::StackPointer(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl Value {
    // pub fn type_check(ty: &Ty) -> bool {
    //     todo!()
    // }

    /// Returns `true` if the value is [`Int`].
    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(..))
    }

    pub fn as_int(&self) -> Option<&i64> {
        if let Self::Int(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the value is [`Bool`].
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(..))
    }

    pub fn as_bool(&self) -> Option<&bool> {
        if let Self::Bool(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the value is [`StackPointer`].
    pub fn is_stack_pointer(&self) -> bool {
        matches!(self, Self::StackPointer(..))
    }

    pub fn as_stack_pointer(&self) -> Option<&StackPointer> {
        if let Self::StackPointer(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
