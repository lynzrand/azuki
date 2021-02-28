use std::borrow::Cow;

use crate::{Program, TacFunc};
use anymap::AnyMap;

pub mod sanity_checker;

/// Represents a single pass inside the compilation pipeline.
///
/// A `Pass` should be constructible from an [`OptimizeEnvironment`], which
/// supplies external data needed for this pass.
#[allow(unused_variables)]
pub trait Pass {
    /// Returns the name of this pass.
    fn name(&self) -> Cow<str>;

    /// Whether this pass edits the program. This method should correctly
    /// represent the function of [`optimize_program`], since mutable references
    /// are given anyway.
    fn edits_program(&self) -> bool;

    /// Optimize at program level.
    fn optimize_program(&mut self, env: &mut OptimizeEnvironment, program: &mut Program);
}

pub trait FunctionOptimizer {
    /// Returns the name of this pass.
    fn name(&self) -> Cow<str>;

    /// Whether this pass edits the program. This method should correctly
    /// represent the function of [`optimize_program`], since mutable references
    /// are given anyway.
    fn edits_program(&self) -> bool;

    /// Reset this instance for optimizing another function.
    fn reset(&mut self) {}

    /// Optimize a single function.
    fn optimize_func(&mut self, env: &mut OptimizeEnvironment, func: &mut TacFunc);

    /// Perform initialization before any functions are processed.
    fn do_initialization(&mut self, _env: &mut OptimizeEnvironment, _prog: &Program) {}

    /// Perform finalization after all functions are processed.
    fn do_finalization(&mut self, _env: &mut OptimizeEnvironment, _prog: &Program) {}

    /// Transform this optimizer into a pass. You should not overwrite this method
    /// in most cases.
    fn make_pass(self) -> FunctionOptimizerPass<Self>
    where
        Self: Sized,
    {
        FunctionOptimizerPass(self)
    }
}

/// A simple wrapper over a [`FunctionOptimizer`] to create a pass.
pub struct FunctionOptimizerPass<F: ?Sized>(pub F);
impl<F> Pass for FunctionOptimizerPass<F>
where
    F: FunctionOptimizer,
{
    fn name(&self) -> Cow<str> {
        self.0.name()
    }

    fn edits_program(&self) -> bool {
        self.0.edits_program()
    }

    fn optimize_program(&mut self, env: &mut OptimizeEnvironment, program: &mut Program) {
        self.0.do_initialization(env, program);
        for func in program.functions.values_mut() {
            self.0.reset();
            self.0.optimize_func(env, func);
        }
        self.0.do_finalization(env, program);
    }
}

/// The environment of an optimization pass. All data inside this struct will be
/// preserved between passes, allowing passes to retain states here.
#[non_exhaustive]
pub struct OptimizeEnvironment {
    /// External data that passes could save, read or modify.
    pub data: AnyMap,
}

pub struct Pipeline {
    env: OptimizeEnvironment,
    passes: Vec<Box<dyn Pass>>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            env: OptimizeEnvironment {
                data: AnyMap::new(),
            },
            passes: vec![],
        }
    }

    pub fn add_pass<P: Pass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass))
    }

    pub fn add_func_optimizer<F: FunctionOptimizer + 'static>(&mut self, opt: F) {
        self.passes.push(Box::new(FunctionOptimizerPass(opt)))
    }

    pub fn add_pass_boxed(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass)
    }

    pub fn optimize(mut self, program: &mut Program) {
        for pass in &mut self.passes {
            pass.optimize_program(&mut self.env, program)
        }
    }
}
