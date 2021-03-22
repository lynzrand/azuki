//! Passes to ensure that the file is still valid Azuki TAC.

use std::collections::HashMap;

use crate::Program;

use super::FunctionOptimizer;
use bit_set::BitSet;
use smol_str::SmolStr;

#[derive(Debug, Default)]
pub struct SanityChecker {
    decl_vars: BitSet,
    use_vars: BitSet,
}

impl FunctionOptimizer for SanityChecker {
    fn name(&self) -> std::borrow::Cow<str> {
        "sanity-check".into()
    }

    fn edits_program(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.decl_vars.clear();
        self.use_vars.clear();
    }

    fn do_initialization(&mut self, env: &mut super::OptimizeEnvironment, _prog: &Program) {
        env.data.remove::<SanityResult>();
    }

    fn optimize_func(&mut self, env: &mut super::OptimizeEnvironment, func: &mut crate::TacFunc) {
        for (idx, inst) in func.instructions_arena.iter() {
            self.decl_vars.insert(idx.slot() as usize);
            for usage in inst.inst.kind.param_op_iter() {
                self.use_vars.insert(usage.slot() as usize);
            }
        }
        let is_all_vars_declared = self.use_vars.is_subset(&self.decl_vars);

        let mut is_all_jumps_declared = true;
        for (bb_id, bb) in func.basic_block_arena.iter() {
            for target in bb.branch.target_iter() {
                // is_all_jumps_declared |= func.bb_get(target).is_some();
            }
        }

        let entry = env.data.entry().or_insert_with(|| SanityResult {
            is_valid_code: HashMap::new(),
        });

        entry.is_valid_code.insert(
            func.name.clone(),
            is_all_vars_declared && is_all_jumps_declared,
        );
    }
}

pub struct SanityResult {
    is_valid_code: HashMap<SmolStr, bool>,
}

impl SanityResult {
    pub fn is_valid_code(&self) -> &HashMap<SmolStr, bool> {
        &self.is_valid_code
    }
}
