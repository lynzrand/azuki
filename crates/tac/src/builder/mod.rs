//! A builder for constructing SSA functions from regular control flows.
mod func_editor;
use std::ops::{Deref, DerefMut};

use bit_set::BitSet;
pub use func_editor::*;
use petgraph::{graph::EdgeIndex, visit::EdgeRef};
use tinyvec::TinyVec;

use crate::*;

/// A function builder that loosely resembles building SSA functions using the
/// algorithm described in
/// [_Simple and Efficient Construction of Static Single Assignment Form_][ssa]
/// by Matthias Braun _et al._
///
/// [ssa]: https://pp.ipd.kit.edu/uploads/publikationen/braun13cc.pdf
///
/// This type is parameterized by one type, `TVar`, which is the representation
/// of variable in your original language. It is expected to be a small,
/// [`Clone`](std::mem::Clone)
/// [`Ord`](std::cmp::Ord) and [`Debug`](std::Debug) type for usage
/// inside [`BTreeMap`s](BTreeMap)
pub struct FuncBuilder<'a, TVar> {
    /// The function we're building.
    pub editor: FuncEditor<'a>,

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
    variable_map: BTreeMap<TVar, (Ty, BTreeMap<BBId, Index>)>,

    /// Incomplete phi commands (params in our case).
    incomplete_phi: BTreeMap<BBId, Vec<(TVar, Index)>>,
}

impl<'a, TVar> FuncBuilder<'a, TVar>
where
    TVar: Ord + std::fmt::Debug + Clone,
{
    /// Create a function builder for a function with the given `name` and type
    /// undefined for now.
    pub fn new(func_editor: FuncEditor<'a>) -> FuncBuilder<'a, TVar> {
        FuncBuilder {
            editor: func_editor,
            sealed_bbs: BitSet::new(),
            filled_bbs: BitSet::new(),
            variable_map: BTreeMap::new(),
            incomplete_phi: BTreeMap::new(),
        }
    }

    /// Create a function builder for a function with the given `name` and given type `ty`.
    pub fn new_func(func: &'a mut TacFunc) -> FuncBuilder<'a, TVar> {
        Self::new(FuncEditor::new(func))
    }

    /// Build this function.
    ///
    /// ## Panics
    ///
    /// This function panics when there is any basic block not _filled_,
    /// not _sealed_, or there is any incomplete phis lying around.
    pub fn sanity_check(self) {
        for bb in self.editor.func.basic_blocks.node_indices() {
            let bb = bb.index();
            assert!(
                self.filled_bbs.contains(bb),
                "bb{} is not yet filled!\nfunc:\n{}",
                bb,
                self.editor.func
            );
            assert!(
                self.sealed_bbs.contains(bb),
                "bb{} is not yet sealed!\nfunc:\n{}",
                bb,
                self.editor.func
            );
        }
        assert!(
            self.incomplete_phi.is_empty(),
            "there is still incomplete phis: {:?}\nfunc:\n{}",
            self.incomplete_phi,
            self.editor.func
        );
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

        self.sealed_bbs.insert(bb_id.index());
    }

    /// Mark the given basic block as _filled_.
    ///
    /// _Filled_ blocks have all its instructions inserted.
    pub fn mark_filled(&mut self, bb_id: BBId) {
        self.filled_bbs.insert(bb_id.index());
    }

    /// Check if the given basic block is sealed.
    pub fn is_sealed(&self, bb_id: BBId) -> bool {
        self.sealed_bbs.contains(bb_id.index())
    }

    /// Check if the given basic block is filled.
    pub fn is_filled(&self, bb_id: BBId) -> bool {
        self.filled_bbs.contains(bb_id.index())
    }

    pub fn declare_var(&mut self, var: TVar, ty: Ty) {
        self.variable_map.insert(var, (ty, BTreeMap::new()));
    }

    /// Indicate that variable `var` is written as the result of instruction `inst`.
    pub fn write_variable_cur(&mut self, var: TVar, inst: Index) -> TacResult<()> {
        self.write_variable(var, inst, self.current_bb_id())
    }

    /// Indicate that variable `var` is written as the result of instruction `inst`
    /// in basic block `bb_id`. If the variable does not exist, it will be created.
    pub fn write_variable(&mut self, var: TVar, inst: Index, bb_id: BBId) -> TacResult<()> {
        let map = &mut self
            .variable_map
            .get_mut(&var)
            .ok_or_else(|| Error::NoSuchVar(format!("{:?}", var)))?
            .1;
        map.insert(bb_id, inst);
        Ok(())
    }

    /// Indicate that variable `var` is read in the current basic block. Returns the index
    /// to the latest definition of this variable, or `None` if it does not exist.
    ///
    /// ## Side effects
    ///
    /// According to the algorithm, this function may introduce parameters to
    /// `bb` and insert parameter passes to the block's predecessors.
    pub fn read_variable_cur(&mut self, var: TVar) -> Option<Index> {
        self.read_variable(var, self.current_bb_id())
    }

    /// Indicate that variable `var` is read in basic block `bb`. Returns the index
    /// to the latest definition of this variable, or `None` if it does not exist.
    ///
    /// ## Side effects
    ///
    /// According to the algorithm, this function may introduce parameters to
    /// `bb` and insert parameter passes to the block's predecessors.
    pub fn read_variable(&mut self, var: TVar, bb_id: BBId) -> Option<Index> {
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
    fn read_variable_recursive(&mut self, var: TVar, bb_id: BBId) -> Option<Index> {
        let var_ty = self.variable_map.get(&var)?.0.clone();
        let val = if !self.sealed_bbs.contains(bb_id.index()) {
            let param = self.editor.insert_param(bb_id, var_ty).unwrap();

            let block = self.incomplete_phi.entry(bb_id).or_insert_with(Vec::new);
            block.push((var.clone(), param));

            param
        } else {
            let preds = self.pred_of_bb(bb_id);
            if preds.len() == 1 {
                self.read_variable(var.clone(), preds[0])?
            } else {
                let inst = self.editor.insert_param(bb_id, var_ty).unwrap();
                self.write_variable(var.clone(), inst, bb_id).unwrap();
                self.add_phi_operands(var.clone(), inst, bb_id, &preds);
                inst
            }
        };

        self.write_variable(var, val, bb_id).unwrap();
        Some(val)
    }

    /// This function directly corresponds to `addPhiOperands` in the algorithm.
    fn add_phi_operands(&mut self, var: TVar, phi: Index, current_bb: BBId, preds: &[BBId]) {
        for &pred in preds {
            let source = self.read_variable(var.clone(), pred).unwrap();
            let bb = self.editor.func.basic_blocks.node_weight_mut(pred).unwrap();
            bb.jumps
                .iter_mut()
                .for_each(|x| x.add_param(current_bb, phi, source));
        }
        self.try_remove_trivial_phi(phi);
    }

    fn try_remove_trivial_phi(&mut self, phi_op: Index) {
        let mut same = None;
        let phi = self.editor.func.arena_get(phi_op).unwrap();
        let phi_bb = phi.bb;
        let preds = self.pred_of_bb(phi_bb);

        // for op in phi.operands:
        'bb: for &bb in &preds {
            let bb = self.editor.func.basic_blocks.node_weight(bb).unwrap();
            for branch in bb
                .jumps
                .iter()
                .filter_map(|b| b.target())
                .filter(|t| t.bb == phi_bb)
            {
                let operand = *branch
                    .params
                    .get(&phi_op)
                    .expect("Phi operation should reside in bb's direct predecessor");
                // if op == same || op == phi
                if operand == phi_op || same.map_or(false, |x| x == operand) {
                    continue 'bb;
                }
                if same.is_some() {
                    // not trivial
                    return;
                }
                same = Some(operand);
            }
        }
        let replace_value = match same {
            None => InstKind::Dead,
            Some(same) => InstKind::Assign(same),
        };
        // remove traces of this phi
        for &bb in &preds {
            let bb = self.editor.func.basic_blocks.node_weight_mut(bb).unwrap();
            for branch in bb
                .jumps
                .iter_mut()
                .filter_map(|b| b.target_mut())
                .filter(|t| t.bb == phi_bb)
            {
                branch.params.remove(&phi_op);
            }
        }
        // FIXME: workaround for not being able to track a phi's users
        // replace usage of this phi
        self.editor.func.arena_get_mut(phi_op).unwrap().inst.kind = replace_value;
    }
}

impl<'a, TVar> Deref for FuncBuilder<'a, TVar> {
    type Target = FuncEditor<'a>;

    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl<'a, TVar> DerefMut for FuncBuilder<'a, TVar> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.editor
    }
}

type SmallBBIdVec = tinyvec::TinyVec<[BBId; 7]>;
type SmallEdgeVec = tinyvec::TinyVec<[EdgeIndex; 7]>;
