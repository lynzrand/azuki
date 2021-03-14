use std::collections::BTreeMap;

use petgraph::visit::EdgeRef;
use tinyvec::TinyVec;

use crate::{
    err::{Error, TacResult},
    BBId, BasicBlock, Branch, Inst, InstId, InstKind, Tac, TacFunc, Ty,
};

use super::{SmallBBIdVec, SmallEdgeVec};

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
    /// **This value MUST refer to an instruction inside [`current_bb`](Self::current_bb).**
    /// **If this value is [`None`](Option::None), `current_bb` MUST be empty.**
    current_idx: Option<InstId>,
}

impl<'a> FuncEditor<'a> {
    pub fn new(func: &'a mut TacFunc) -> FuncEditor<'a> {
        let starting_block = func.bb_seq.first().cloned().unwrap_or_default();
        let starting_idx = func.bb_get(starting_block).head;
        let current_bb = starting_block;

        FuncEditor {
            func,
            current_bb_id: current_bb,
            current_idx: starting_idx,
        }
    }

    /// Create a new function editor with ABSOLUTELY nothing initialized.
    ///
    /// PLEASE DEFINITELY REMEMBER TO INITIALIZE BEFORE PUTTING ANYTHING INSIDE
    pub fn new_blank(func: &'a mut TacFunc) -> FuncEditor<'a> {
        let starting_idx = None;
        let current_bb = func.bb_seq.first().cloned().unwrap_or_default();

        FuncEditor {
            func,
            current_bb_id: current_bb,
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

    /// Add an free-standing empty basic block into the function.
    pub fn new_bb(&mut self) -> BBId {
        let bb = self
            .func
            .basic_block_arena
            .insert(BasicBlock {
                jumps: vec![],
                head: None,
                tail: None,
            })
            .into();
        self.func.bb_seq.push(bb);
        bb
    }

    /// Set current basic block to `bb_id`. Also sets [`current_idx`](Self::current_idx)
    /// to the end of this basic block.
    ///
    /// Returns whether the position was **unchanged**.
    pub fn set_current_bb(&mut self, bb_id: BBId) -> TacResult<bool> {
        let bb = self.func.bb_get(bb_id);
        let same_pos = bb_id == self.current_bb_id && bb.tail == self.current_idx;
        self.current_bb_id = bb_id;
        self.current_idx = bb.tail;
        Ok(same_pos)
    }

    /// Set current basic block to `bb_id`. Also sets [`current_idx`](Self::current_idx)
    /// to the start of this basic block.
    ///
    /// Returns whether the position was **unchanged**.
    pub fn set_current_bb_start(&mut self, bb_id: BBId) -> TacResult<bool> {
        let bb = self.func.bb_get(bb_id);
        let same_pos = bb_id == self.current_bb_id && bb.head == self.current_idx;
        self.current_bb_id = bb_id;
        self.current_idx = bb.head;
        Ok(same_pos)
    }

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
        self.put_inst_after_current_place(idx).unwrap();
        idx
    }

    /// Insert the given instruction **before** the current place. Returns the index to
    /// the inserted instruction (and also the SSA value it's related to).
    ///
    /// If the current basic block is empty, the instruction is inserted as the
    /// only instruction of the basic block.
    pub fn insert_before_current_place(&mut self, inst: Inst) -> InstId {
        let idx = self.func.inst_new(inst);
        self.put_inst_before_current_place(idx).unwrap();
        idx
    }

    /// Insert the given instruction at the **end** of the given basic block.
    pub fn insert_at_end_of(&mut self, inst: Inst, bb_id: BBId) -> TacResult<InstId> {
        let curr_bb = self.current_bb_id;
        let curr_idx = self.current_idx;
        let same_pos = self.set_current_bb(bb_id)?;
        let insert_pos = self.insert_after_current_place(inst);
        if !same_pos {
            self.current_bb_id = curr_bb;
            self.current_idx = curr_idx;
        }
        Ok(insert_pos)
    }

    /// Insert the given instruction at the **start** of the given basic block.
    pub fn insert_at_start_of(&mut self, inst: Inst, bb_id: BBId) -> TacResult<InstId> {
        let curr_bb = self.current_bb_id;
        let curr_idx = self.current_idx;
        let same_pos = self.set_current_bb_start(bb_id)?;
        let insert_pos = self.insert_before_current_place(inst);
        if !same_pos {
            self.current_bb_id = curr_bb;
            self.current_idx = curr_idx;
        }
        Ok(insert_pos)
    }

    /// Attach the free-standing instruction to the place after [`current_idx`],
    /// and advance one instruction forward.
    ///
    /// # Panics
    ///
    /// Panics when the instruction is not free-standing (`inst.prev` or
    /// `inst.next` is not [`None`]).
    pub fn put_inst_after_current_place(&mut self, idx: InstId) -> TacResult<()> {
        {
            let inst = self.func.tac_get(idx);
            assert_eq!(inst.prev, None);
            assert_eq!(inst.next, None);
        }
        if let Some(cur_idx) = self.current_idx {
            self.func.inst_set_after(cur_idx, idx);
            let bb = self.func.bb_get_mut(self.current_bb_id);

            // reset tail pointer, since insertion might be at the end
            if bb.tail == Some(cur_idx) {
                bb.tail = Some(idx);
            }
        } else {
            let bb = self.func.bb_get_mut(self.current_bb_id);
            bb.head = Some(idx);
            bb.tail = Some(idx);
        }
        self.current_idx = Some(idx);
        Ok(())
    }

    /// Attach the free-standing instruction to the place before [`current_idx`],
    /// and advance one instruction back.
    ///
    /// # Panics
    ///
    /// Panics when the instruction is not free-standing (`inst.prev` or
    /// `inst.next` is not [`None`]).
    fn put_inst_before_current_place(&mut self, idx: InstId) -> TacResult<()> {
        {
            let inst = self.func.tac_get(idx);
            assert_eq!(inst.prev, None);
            assert_eq!(inst.next, None);
        }
        if let Some(cur_idx) = self.current_idx {
            self.func.inst_set_before(cur_idx, idx);
            let bb = self.func.bb_get_mut(self.current_bb_id);

            // reset head pointer, since insertion might be at the start
            if bb.head == self.current_idx {
                bb.head = Some(idx);
            }
        } else {
            let bb = self.func.bb_get_mut(self.current_bb_id);
            bb.head = Some(idx);
            bb.tail = Some(idx);
        }
        self.current_idx = Some(idx);
        Ok(())
    }

    /// Add a branching instruction to the given basic block's jump instruction list.
    pub fn add_branch(&mut self, inst: Branch, bb_id: BBId) -> TacResult<()> {
        for target in inst.target_iter() {
            self.func.basic_blocks_graph.add_edge(bb_id, target, ());
        }

        let bb = self.func.bb_get_mut(bb_id);

        bb.jumps.push(inst);

        Ok(())
    }

    /// Modifies the branching instructions of a basic block. Recalculates successors of this
    /// basic block after the modification completes.
    pub fn modify_branch<F: FnOnce(&mut Vec<Branch>)>(
        &mut self,
        bb_id: BBId,
        f: F,
    ) -> TacResult<()> {
        for target in self.succ_of_bb(bb_id) {
            self.func.basic_blocks_graph.remove_edge(bb_id, target);
        }

        let bb = self.func.bb_get_mut(bb_id);

        f(&mut bb.jumps);

        for target in bb
            .jumps
            .iter()
            .flat_map(|x| x.target_iter())
            .collect::<TinyVec<[_; 16]>>()
        {
            self.func.basic_blocks_graph.add_edge(bb_id, target, ());
        }

        Ok(())
    }

    /// Returns an iterator of all predecessors of a basic block.
    ///
    /// The return type is to make the borrow checker happy.
    pub fn pred_of_bb(&self, bb_id: BBId) -> SmallBBIdVec {
        self.func
            .basic_blocks_graph
            .neighbors_directed(bb_id, petgraph::Direction::Incoming)
            .collect()
    }

    /// Returns an iterator of all successors of a basic block.
    pub fn succ_of_bb(&self, bb_id: BBId) -> SmallBBIdVec {
        self.func
            .basic_blocks_graph
            .neighbors_directed(bb_id, petgraph::Direction::Outgoing)
            .collect()
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

    /// Move one instruction forward. Returns whether the move was successful.
    /// If this function returns `true`, [`current_idx`] and functions related
    /// to it are guaranteed to return `Some` as long as .
    pub fn move_forward(&mut self) -> bool {
        if let Some(inst) = self.current_tac() {
            if inst.next.is_some() {
                self.current_idx = inst.next;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Move one instruction backward. Returns whether the move was successful.
    /// If this function returns `true`, [`current_idx`] and functions related
    /// to it are guaranteed to return `Some`.
    pub fn move_backward(&mut self) -> bool {
        if let Some(inst) = self.current_tac() {
            if inst.prev.is_some() {
                self.current_idx = inst.prev;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn remove_current(&mut self) -> Option<Inst> {
        if let Some(idx) = self.current_idx {
            Some(self.func.inst_remove(idx))
        } else {
            None
        }
    }
}
