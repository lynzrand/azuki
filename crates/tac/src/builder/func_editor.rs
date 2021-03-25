use std::collections::BTreeMap;

use crate::{
    err::{Error, TacResult},
    BBId, BasicBlock, Inst, InstId, InstKind, Tac, TacFunc, Ty,
};

/// An editor attached to the given function for linear editing purposes.
pub struct FuncEditor<'a> {
    pub func: &'a mut TacFunc,

    /// The basic block we're currently working on. Must be a valid basic block
    /// inside this function.
    current_bb_id: BBId,

    /// The instruction index we're currently working on. New instructions will
    /// be inserted before or after this instruction, depending on the function
    /// we use.
    ///
    /// If this value is `Some(inst)`, inst MUST point to a valid instruction
    /// inside [`current_bb`]. If this value is `None`, either it points at a
    /// sentinel position that has `bb.head` as next and `bb.tail` as prev,
    /// or the basic block is completely empty.
    current_idx: Option<InstId>,
}

impl<'a> FuncEditor<'a> {
    /// Create a new function editor with ABSOLUTELY nothing initialized.
    ///
    /// PLEASE DEFINITELY REMEMBER TO INITIALIZE BEFORE PUTTING ANYTHING INSIDE
    pub fn new(func: &'a mut TacFunc) -> FuncEditor<'a> {
        let current_bb = func.first_block;
        let starting_idx = current_bb.and_then(|b| func.bb_get(b).head);

        FuncEditor {
            func,
            current_bb_id: current_bb.unwrap_or_default(),
            current_idx: starting_idx,
        }
    }

    pub fn set_type(&mut self, ty: Ty) {
        self.func.ty = ty;
    }

    /// Returns the current basic block this builder is working on.
    pub fn current_bb_id(&self) -> BBId {
        self.current_bb_id
    }

    pub fn current_bb(&self) -> &BasicBlock {
        self.func.bb_get(self.current_bb_id)
    }

    pub fn current_bb_mut(&mut self) -> &mut BasicBlock {
        self.func.bb_get_mut(self.current_bb_id)
    }

    /// Returns the current instruction this builder is working on. If
    /// [`current_bb`](Self::current_bb) is empty, returns [`None`](Option::None).
    pub fn current_idx(&self) -> Option<InstId> {
        self.current_idx
    }

    pub fn current_inst(&self) -> Option<&Inst> {
        self.current_tac().map(|x| &x.inst)
    }

    pub fn current_tac(&self) -> Option<&Tac> {
        Some(self.func.tac_get(self.current_idx?))
    }

    pub fn current_inst_mut(&mut self) -> Option<&mut Inst> {
        self.current_tac_mut().map(|x| &mut x.inst)
    }

    pub fn current_tac_mut(&mut self) -> Option<&mut Tac> {
        Some(self.func.tac_get_mut(self.current_idx?))
    }

    /// Add an empty basic block into the function.
    pub fn new_bb(&mut self) -> BBId {
        self.func.bb_new()
    }

    /// Set current basic block to `bb_id`. Also sets [`current_idx`](Self::current_idx)
    /// to the sentinel value that moves backward to the last instruction, and
    /// forward to the first instruction.
    ///
    /// Returns whether the position was **unchanged**.
    ///
    /// # Note
    ///
    /// The reason why this function sets the position at a sentinel value
    /// is to ensure one can use [`move_forward`] as the condition of a while
    /// loop to visit all instructions in this function.
    pub fn set_current_bb(&mut self, bb_id: BBId) -> bool {
        debug_assert!(self.func.bb_exists(bb_id));
        let same_pos = bb_id == self.current_bb_id && None == self.current_idx;
        self.current_bb_id = bb_id;
        self.current_idx = None;
        same_pos
    }

    // /// Set current basic block to `bb_id`. Also sets [`current_idx`](Self::current_idx)
    // /// to the start of this basic block.
    // ///
    // /// Returns whether the position was **unchanged**.
    // pub fn set_current_bb_start(&mut self, bb_id: BBId) -> bool {
    //     debug_assert!(self.func.bb_exists(bb_id));
    //     let bb = self.func.bb_get(bb_id);
    //     let same_pos = bb_id == self.current_bb_id && bb.head == self.current_idx;
    //     self.current_bb_id = bb_id;
    //     self.current_idx = bb.head;
    //     same_pos
    // }

    /// Sets current basic block and instruction position at the position of the
    /// given instruction.
    ///
    /// Returns whether the position was **unchanged**.
    pub fn set_position_at_instruction(&mut self, inst_idx: InstId) -> TacResult<bool> {
        let inst = self.func.tac_get(inst_idx);
        let bb = inst.bb;
        let same_pos = bb == self.current_bb_id && Some(inst_idx) == self.current_idx;
        self.current_bb_id = bb;
        self.current_idx = Some(inst_idx);
        Ok(same_pos)
    }

    /// Insert the given instruction **after** the current place. Returns the index to
    /// the inserted instruction (and also the SSA value it's related to).
    ///
    /// If the current basic block is empty, the instruction is inserted as the
    /// only instruction of the basic block.
    pub fn insert_after_current_place(&mut self, inst: Inst) -> InstId {
        let idx = self.func.inst_new(inst);
        // this line is infailable
        self.put_inst_after_current_place(idx);
        idx
    }

    /// Insert the given instruction **before** the current place. Returns the index to
    /// the inserted instruction (and also the SSA value it's related to).
    ///
    /// If the current basic block is empty, the instruction is inserted as the
    /// only instruction of the basic block.
    pub fn insert_before_current_place(&mut self, inst: Inst) -> InstId {
        let idx = self.func.inst_new(inst);
        self.put_inst_before_current_place(idx);
        idx
    }

    /// Insert the given instruction at the **end** of the given basic block.
    pub fn insert_at_end_of(&mut self, inst: Inst, bb_id: BBId) -> TacResult<InstId> {
        let inst = self.func.inst_new(inst);
        self.func.inst_append_in_bb(inst, bb_id);
        Ok(inst)
    }

    /// Insert the given instruction at the **start** of the given basic block.
    pub fn insert_at_start_of(&mut self, inst: Inst, bb_id: BBId) -> TacResult<InstId> {
        let inst = self.func.inst_new(inst);
        self.func.inst_prepend_in_bb(inst, bb_id);
        Ok(inst)
    }

    /// Attach the free-standing instruction to the place after [`current_idx`],
    /// and advance one instruction forward.
    ///
    /// If `current_idx` is `None`, the instruction will be **prepended** in the
    /// basic block (since `None` refers to the sentinel position).
    ///
    /// # Panics
    ///
    /// Panics when the instruction is not free-standing (`inst.prev` or
    /// `inst.next` is not [`None`]).
    pub fn put_inst_after_current_place(&mut self, idx: InstId) {
        if let Some(cur) = self.current_idx {
            self.func.inst_set_after(cur, idx);
        } else {
            self.func.inst_prepend_in_bb(idx, self.current_bb_id);
        }
        self.current_idx = Some(idx);
    }

    /// Attach the free-standing instruction to the place before [`current_idx`],
    /// and advance one instruction back.
    ///
    /// If `current_idx` is `None`, the instruction will be **appended** in the
    /// basic block (since `None` refers to the sentinel position).
    ///
    /// # Panics
    ///
    /// Panics when the instruction is not free-standing (`inst.prev` or
    /// `inst.next` is not [`None`]).
    fn put_inst_before_current_place(&mut self, idx: InstId) {
        if let Some(cur) = self.current_idx {
            self.func.inst_set_before(cur, idx);
        } else {
            self.func.inst_append_in_bb(idx, self.current_bb_id);
        }
        self.current_idx = Some(idx);
    }

    pub fn insert_phi(&mut self, bb_id: BBId, ty: Ty) -> Result<InstId, Error> {
        self.insert_at_start_of(
            Inst {
                kind: InstKind::Phi(BTreeMap::new()),
                ty,
            },
            bb_id,
        )
    }

    /// Move one instruction forward. Returns whether [`current_idx`] is a valid
    /// instruction value.
    ///
    /// If `current_idx` is `None`, this will move to the first instruction in
    /// the basic block, if possible.
    pub fn move_forward(&mut self) -> bool {
        if let Some(inst) = self.current_tac() {
            let next = inst.next;
            self.current_idx = next;
        } else {
            let bb = self.current_bb();
            self.current_idx = bb.head;
        }
        self.current_idx.is_some()
    }

    /// Move one instruction backward. Returns whether [`current_idx`] is a valid
    /// instruction value.
    ///
    /// If `current_idx` is `None`, this will move to the last instruction in
    /// the basic block, if possible.
    pub fn move_backward(&mut self) -> bool {
        if let Some(inst) = self.current_tac() {
            let next = inst.prev;
            self.current_idx = next;
        } else {
            let bb = self.current_bb();
            self.current_idx = bb.tail;
        }
        self.current_idx.is_some()
    }

    pub fn remove_current(&mut self) -> (bool, Option<Inst>) {
        if let Some(idx) = self.current_idx {
            let has_next = self.move_forward();
            self.func.inst_detach(idx);
            (has_next, Some(self.func.inst_remove(idx)))
        } else {
            (false, None)
        }
    }
}
