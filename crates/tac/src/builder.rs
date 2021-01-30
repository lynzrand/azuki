//! A builder for constructing SSA functions from regular control flows.

use bit_set::BitSet;

use crate::*;

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
    /// This graph is in fact calculable from `self.func`, but having an organized
    /// version here with graph search support makes everything easier.
    cfg: petgraph::graphmap::DiGraphMap<usize, ()>,

    /// Sealed basic blocks.
    ///
    /// Sealed basic blocks have all their predecessors determined.
    sealed_bbs: BitSet,

    /// Filled basic blocks.
    ///
    /// Filled basic blocks have finished filling in calculation instructions.
    filled_bbs: BitSet,

    /// A `Variable` - `Basic Block` - `SSA Value` map.
    ///
    /// This map is for global and local value numbering in the algorithm. Since
    /// we expect variables and basic blocks to be small integer IDs, `BTreeMap` is
    /// preferred here over `HashMap`.
    ///
    /// Well... You see, there's a [forest][] in cranelift, right?
    ///
    /// [forest]: https://github.com/bytecodealliance/wasmtime/tree/HEAD/cranelift/bforest
    variable_map: BTreeMap<VarId, (Ty, BTreeMap<BBId, Index>)>,

    /// Incomplete phi commands (params in our case).
    incomplete_phi: BTreeMap<BBId, Vec<(VarId, Index)>>,
}

impl FuncBuilder {
    /// Create a function builder for a function with the given `name`.
    pub fn new(name: SmolStr) -> FuncBuilder {
        let mut f = TacFunc::new(name);

        f.basic_blocks.insert(
            0,
            BasicBlock {
                head: None,
                tail: None,
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
            variable_map: BTreeMap::new(),
            incomplete_phi: BTreeMap::new(),
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
                head: None,
                tail: None,
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
        self.current_idx = bb.tail;
        Ok(())
    }

    /// Set current basic block to `bb_id`. Also sets [`current_idx`](Self::current_idx)
    /// to the start of this basic block.
    pub fn set_current_bb_start(&mut self, bb_id: BBId) -> TacResult<()> {
        let bb = self
            .func
            .basic_blocks
            .get(&bb_id)
            .ok_or(Error::NoSuchBB(bb_id))?;
        self.current_bb = bb_id;
        self.current_idx = bb.head;
        Ok(())
    }

    /// Sets current basic block and instruction position at the position of the given instruction.
    pub fn set_position_at_instruction(&mut self, inst_idx: Index) -> TacResult<()> {
        let inst = self.func.arena_get(inst_idx)?;
        let bb = inst.bb;
        self.current_bb = bb;
        self.current_idx = Some(inst_idx);
        Ok(())
    }

    /// Mark the given basic block as _sealed_. Also completes all incomplete Phi commands
    /// inside this basic block.
    ///
    /// _Sealed_ blocks have all its predecessors determined.
    pub fn mark_sealed(&mut self, bb_id: BBId) {
        if let Some(phis) = self.incomplete_phi.remove(&bb_id) {
            let bb_preds = self.pred_of_bb(bb_id);
            for (var, phi) in phis {
                self.add_phi_operands(var, phi, bb_id, &bb_preds);
            }
        }

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

    pub fn declare_var(&mut self, var: usize, ty: Ty) {
        self.variable_map.insert(var, (ty, BTreeMap::new()));
    }

    /// Indicate that variable `var` is written as the result of instruction `inst`.
    pub fn write_variable_cur(&mut self, var: usize, inst: Index) -> TacResult<()> {
        self.write_variable(var, inst, self.current_bb())
    }

    /// Indicate that variable `var` is written as the result of instruction `inst`
    /// in basic block `bb_id`. If the variable does not exist, it will be created.
    pub fn write_variable(&mut self, var: usize, inst: Index, bb_id: BBId) -> TacResult<()> {
        let map = &mut self
            .variable_map
            .get_mut(&var)
            .ok_or(Error::NoSuchVar(var))?
            .1;
        map.insert(bb_id, inst);
        Ok(())
    }

    /// Indicate that variable `var` is read in basic block `bb`. Returns the index
    /// to the latest definition of this variable, or `None` if it does not exist.
    ///
    /// ## Side effects
    ///
    /// According to the algorithm, this function may introduce parameters to
    /// `bb` and insert parameter passes to the block's predecessors.
    pub fn read_variable(&mut self, var: usize, bb_id: BBId) -> Option<Index> {
        let subtree = &self.variable_map.get(&var)?.1;
        if let Some(idx) = subtree.get(&bb_id) {
            // local numbering works!
            Some(*idx)
        } else {
            // search in predecessors, aka global numbering
            self.read_variable_recursive(var, bb_id)
        }
    }

    /// This function directly corresponds to `readVariableRecursive` in the algorithm.
    fn read_variable_recursive(&mut self, var: usize, bb_id: BBId) -> Option<Index> {
        let var_ty = self.variable_map.get(&var)?.0.clone();
        let val = if !self.sealed_bbs.contains(bb_id) {
            let param = self
                .insert_at_start_of(
                    Inst {
                        kind: InstKind::Param,
                        ty: var_ty,
                    },
                    bb_id,
                )
                .unwrap();

            let block = self.incomplete_phi.entry(bb_id).or_insert_with(Vec::new);
            block.push((var, param));

            param
        } else {
            let preds = self.pred_of_bb(bb_id);
            if preds.len() == 1 {
                self.read_variable(var, bb_id)?
            } else {
                let inst = self
                    .insert_at_start_of(
                        Inst {
                            kind: InstKind::Param,
                            ty: var_ty,
                        },
                        bb_id,
                    )
                    .unwrap();
                self.write_variable(var, inst, bb_id).unwrap();
                self.add_phi_operands(var, inst, bb_id, &preds);
                inst
            }
        };

        self.write_variable(var, val, bb_id).unwrap();
        Some(val)
    }

    /// This function directly corresponds to `addPhiOperands` in the algorithm.
    fn add_phi_operands(
        &mut self,
        var: usize,
        target_inst: Index,
        current_bb: BBId,
        preds: &[BBId],
    ) {
        for &pred in preds {
            let source = self.read_variable(var, pred).unwrap();
            let bb = self.func.basic_blocks.get_mut(&pred).unwrap();
            bb.jumps.add_param(current_bb, target_inst, source);
        }
        // TODO: TryRemoveTrivialPhi()
    }

    /// Insert the given instruction **after** the current place. Returns the index to
    /// the inserted instruction (and also the SSA value it's related to).
    ///
    /// If the current basic block is empty, the instruction is inserted as the
    /// only instruction of the basic block.
    pub fn insert_after_current_place(&mut self, inst: Inst) -> Index {
        let idx = self.func.tac_new(inst, self.current_bb());
        if let Some(cur_idx) = self.current_idx {
            self.func.tac_set_after(cur_idx, idx).unwrap();
            let bb = self.func.basic_blocks.get_mut(&self.current_bb).unwrap();

            // reset tail pointer, since insertion might be at the end
            if bb.tail == Some(cur_idx) {
                bb.tail = Some(idx);
            }
        } else {
            let bb = self.func.basic_blocks.get_mut(&self.current_bb).unwrap();
            bb.head = Some(idx);
            bb.tail = Some(idx);
        }
        self.current_idx = Some(idx);
        idx
    }

    /// Insert the given instruction **before** the current place. Returns the index to
    /// the inserted instruction (and also the SSA value it's related to).
    ///
    /// If the current basic block is empty, the instruction is inserted as the
    /// only instruction of the basic block.
    pub fn insert_before_current_place(&mut self, inst: Inst) -> Index {
        let idx = self.func.tac_new(inst, self.current_bb());
        if let Some(cur_idx) = self.current_idx {
            self.func.tac_set_before(cur_idx, idx).unwrap();
            let bb = self.func.basic_blocks.get_mut(&self.current_bb).unwrap();

            // reset head pointer, since insertion might be at the start
            if bb.head == Some(cur_idx) {
                bb.head = Some(idx);
            }
        } else {
            let bb = self.func.basic_blocks.get_mut(&self.current_bb).unwrap();
            bb.head = Some(idx);
            bb.tail = Some(idx);
        }
        self.current_idx = Some(idx);
        idx
    }

    /// Insert the given instruction at the **end** of the given basic block.
    pub fn insert_at_end_of(&mut self, inst: Inst, bb_id: BBId) -> TacResult<Index> {
        let curr_bb = self.current_bb;
        let curr_idx = self.current_idx;
        self.set_current_bb(bb_id)?;
        let insert_pos = self.insert_after_current_place(inst);
        self.current_bb = curr_bb;
        self.current_idx = curr_idx;
        Ok(insert_pos)
    }

    /// Insert the given instruction at the **start** of the given basic block.
    pub fn insert_at_start_of(&mut self, inst: Inst, bb_id: BBId) -> TacResult<Index> {
        let curr_bb = self.current_bb;
        let curr_idx = self.current_idx;
        self.set_current_bb_start(bb_id)?;
        let insert_pos = self.insert_before_current_place(inst);
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

        let orig = std::mem::replace(&mut bb.jumps, inst);

        // TODO: Should we make jump instructions a list?

        for target in orig.iter() {
            self.cfg.remove_edge(bb_id, target);
        }
        for target in bb.jumps.iter() {
            self.cfg.add_edge(bb_id, target, ());
        }
        Ok(match orig {
            Branch::Unreachable => None,
            a => Some(a),
        })
    }

    /// Returns an iterator of all predecessors of a basic block.
    ///
    /// The return type is to make the borrow checker happy.
    pub fn pred_of_bb(&self, bb_id: BBId) -> SmallBBIdVec {
        self.cfg
            .neighbors_directed(bb_id, petgraph::Direction::Incoming)
            .collect()
    }

    /// Returns an iterator of all successors of a basic block.
    pub fn succ_of_bb(&self, bb_id: BBId) -> SmallBBIdVec {
        self.cfg
            .neighbors_directed(bb_id, petgraph::Direction::Outgoing)
            .collect()
    }
}

type SmallBBIdVec = tinyvec::TinyVec<[BBId; 7]>;
