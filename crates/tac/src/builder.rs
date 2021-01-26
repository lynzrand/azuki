use bit_set::BitSet;

use crate::*;

/// The index of a basic block.
type BBId = usize;

/// The index of an external variable.
type VarId = usize;

/// A function builder that loosely resembles building SSA functions using the
/// algorithm described in
/// [_Simple and Efficient Construction of Static Single Assignment Form_][ssa]
/// by Matthias Braun _et al._
///
/// [ssa]: https://pp.ipd.kit.edu/uploads/publikationen/braun13cc.pdf
pub struct FuncBuilder {
    /// The function we're building.
    func: TacFunc,

    /// Total count of basic blocks. Basic blocks (BBs) are sequentially numbered
    /// from 0. A function always starts from BB0.
    bb_count: BBId,

    /// The basic block we're currently working on. Must be a valid basic block
    /// inside this function.
    current_bb: BBId,

    /// The instruction index we're currently working on. New instructions will
    /// be inserted before or after this instruction, depending on the function
    /// we use.
    ///
    /// **This value MUST refer to an instruction inside [`current_bb`](Self::current_bb).**
    /// **If this value is [`None`](Option::None), `current_bb` MUST be empty.**
    current_idx: Option<Index>,

    /// Control flow graph
    ///
    /// This graph is in fact embedded inside `self.func`, but having an identical
    /// version here with `petgraph` makes everything easier.
    cfg: petgraph::graphmap::DiGraphMap<usize, ()>,

    /// Sealed basic blocks.
    ///
    /// Sealed basic blocks have all their predecessors determined.
    sealed_bbs: BitSet,

    /// Filled basic blocks.
    ///
    /// Filled basic blocks have finished filling in calculation instructions.
    filled_bbs: BitSet,
}

impl FuncBuilder {
    /// Create a function builder for a function with the given `name`.
    pub fn new(name: SmolStr) -> FuncBuilder {
        let mut f = TacFunc::new(name);

        f.basic_blocks.insert(
            0,
            BasicBlock {
                op_start: None,
                op_end: None,
                params: None,
                jumps: Default::default(),
            },
        );

        FuncBuilder {
            func: f,
            cfg: petgraph::graphmap::DiGraphMap::new(),
            current_bb: 0,
            current_idx: None,
            bb_count: 0,
            sealed_bbs: BitSet::new(),
            filled_bbs: BitSet::new(),
        }
    }

    /// Build this function.
    pub fn build(self) -> TacFunc {
        self.func
    }

    /// Returns the current basic block this builder is working on.
    pub fn current_bb(&self) -> BBId {
        self.current_bb
    }

    /// Returns the current instruction this builder is working on. If
    /// [`current_bb`](Self::current_bb) is empty, returns [`None`](Option::None).
    pub fn current_idx(&self) -> Option<Index> {
        self.current_idx
    }

    /// Add an free-standing empty basic block into the function.
    pub fn new_bb(&mut self) -> BBId {
        let bb_id = self.bb_count;
        self.bb_count += 1;
        self.func.basic_blocks.insert(
            bb_id,
            BasicBlock {
                params: None,
                jumps: Branch::Unreachable,
                op_start: None,
                op_end: None,
            },
        );
        self.cfg.add_node(bb_id);
        bb_id
    }

    /// Set current basic block to `bb_id`. Also sets [`current_idx`](Self::current_idx)
    /// to the end of this basic block.
    pub fn set_current_bb(&mut self, bb_id: BBId) -> TacResult<()> {
        let bb = self
            .func
            .basic_blocks
            .get(&bb_id)
            .ok_or(Error::NoSuchBB(bb_id))?;
        self.current_bb = bb_id;
        self.current_idx = bb.op_end;
        Ok(())
    }

    /// Mark the given basic block as _sealed_.
    ///
    /// _Sealed_ blocks have all its predecessors determined.
    pub fn mark_sealed(&mut self, bb_id: BBId) {
        self.sealed_bbs.insert(bb_id);
    }

    /// Mark the given basic block as _filled_.
    ///
    /// _Filled_ blocks have all its instructions inserted.
    pub fn mark_filled(&mut self, bb_id: BBId) {
        self.filled_bbs.insert(bb_id);
    }

    /// Check if the given basic block is sealed.
    pub fn is_sealed(&self, bb_id: BBId) -> bool {
        self.sealed_bbs.contains(bb_id)
    }

    /// Check if the given basic block is filled.
    pub fn is_filled(&self, bb_id: BBId) -> bool {
        self.filled_bbs.contains(bb_id)
    }

    /// Indicate that variable `var` is written as the result of instruction `inst`.
    pub fn write_variable_curr(&mut self, var: usize, inst: Index) {
        self.write_variable(var, inst, self.current_bb())
    }

    /// Indicate that variable `var` is written as the result of instruction `inst`
    /// in basic block `bb_id`. If the variable does not exist, it will be created.
    pub fn write_variable(&mut self, var: usize, inst: Index, bb_id: BBId) {
        todo!("Write variable")
    }

    /// Indicate that variable `var` is read in basic block `bb`. Returns the index
    /// to the latest definition of this variable, or `None` if it does not exist.
    ///
    /// ## Side effects
    ///
    /// According to the algorithm, this function may introduce parameters to
    /// `bb` and insert parameter passes to the block's predecessors.
    pub fn read_variable(&mut self, var: usize, bb_id: BBId) -> Option<Index> {
        todo!("Read variable / do local numberings")
    }

    /// This function directly corresponds to `readVariableRecursive` in the algorithm.
    fn read_variable_recursive(&mut self, var: usize, bb_id: BBId) -> Option<Index> {
        todo!("do global numberings")
    }

    /// This function directly corresponds to `addPhiOperands` in the algorithm.
    fn add_phi_operands(&mut self, var: usize) {
        todo!()
    }

    /// Insert the given instruction after the current place. Returns the index to
    /// the inserted instruction (and also the SSA value it's related to).
    pub fn insert_after_current_place(&mut self, inst: Inst) -> Index {
        let idx = self.func.tac_new(inst);
        if let Some(cur_idx) = self.current_idx {
            self.func.tac_set_after(cur_idx, idx).unwrap();
            let bb = self.func.basic_blocks.get_mut(&self.current_bb).unwrap();
            if bb.op_end == Some(cur_idx) {
                bb.op_end = Some(idx);
            }
        } else {
            let bb = self.func.basic_blocks.get_mut(&self.current_bb).unwrap();
            bb.op_start = Some(idx);
            bb.op_end = Some(idx);
        }
        self.current_idx = Some(idx);
        idx
    }

    /// Insert the given instruction at the end of the given basic block.
    pub fn insert_at_end_of(&mut self, inst: Inst, bb_id: BBId) -> TacResult<Index> {
        let curr_bb = self.current_bb;
        let curr_idx = self.current_idx;
        self.set_current_bb(bb_id)?;
        let insert_pos = self.insert_after_current_place(inst);
        self.current_bb = curr_bb;
        self.current_idx = curr_idx;
        Ok(insert_pos)
    }

    /// Set the jump instruction of the given basic block. Returns the old jump instruction if it's
    /// not [`Unreachable`](Branch::Unreachable).
    pub fn set_jump_inst(&mut self, inst: Branch, bb_id: BBId) -> TacResult<Option<Branch>> {
        let bb = self
            .func
            .basic_blocks
            .get_mut(&bb_id)
            .ok_or(Error::NoSuchBB(bb_id))?;

        for target in inst.iter() {
            self.cfg.add_edge(bb_id, target, ());
        }

        let orig = std::mem::replace(&mut bb.jumps, inst);
        Ok(match orig {
            Branch::Unreachable => None,
            a => Some(a),
        })
    }

    /// Returns an iterator of all predecessors of a basic block.
    pub fn pred_of_bb<'a>(&'a self, bb_id: BBId) -> impl Iterator<Item = BBId> + 'a {
        self.cfg
            .neighbors_directed(bb_id, petgraph::Direction::Incoming)
    }

    /// Returns an iterator of all successors of a basic block.
    pub fn succ_of_bb<'a>(&'a self, bb_id: BBId) -> impl Iterator<Item = BBId> + 'a {
        self.cfg
            .neighbors_directed(bb_id, petgraph::Direction::Outgoing)
    }
}
