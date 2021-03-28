//! Simplifies branches and removes empty basic blocks.
//!
//! # Optimizations
//!
//! Branching simplify performs the follow optimizations:
//!
//! - Replaces `brif _ bbI bbI` into `br bbI`.
//!
//! - Replaces `brif 0 bbI bbJ` into `br bbJ`; replaces `brIf x bbI bbJ` into
//!   `br bbI` where `x` is an immediate and `x != 0`.
//!
//! - Connects basic blocks `bbI` and `bbJ` if `bbI` ends in `br bbJ` and `bbJ`
//!   has only one predecessor.
//!
//! - Removes `bbI` if `bbI` has an empty body and ends in `br bbJ`
//!   (unconditional branch). Replaces all jumps into `bbI` to `bbJ`.
//!
//! - Removes `bbI` if `bbI` has an empty body and all predesessors of `bbI`
//!   is `br bbI` (unconditional branch). Replaces all jumps to `bbI` into
//!   the branching instruction of `bbI`.
//!
//! # Relationship
//!
//! This optimization should follow `const-folding` and be followed by
//! `dead-code-elimination` for best effects.

use azuki_tac::optimizer::FunctionOptimizer;

/// Performs branching simplify. See [module documents](crate::branching_simplify).
pub struct BranchingSimplify;

impl FunctionOptimizer for BranchingSimplify {
    fn name(&self) -> std::borrow::Cow<str> {
        "branching-simplify".into()
    }

    fn edits_program(&self) -> bool {
        true
    }

    fn optimize_func(
        &mut self,
        env: &mut azuki_tac::optimizer::OptimizeEnvironment,
        func: &mut azuki_tac::TacFunc,
    ) {
        todo!()
    }
}
