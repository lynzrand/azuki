use std::borrow::Cow;

use anymap::AnyMap;
use azuki_tac::{Program, TacFunc};

/// Represents a single pass inside the compilation pipeline.
///
/// A `Pass` should be constructible from an [`OptimizeEnvironment`], which
/// supplies external data needed for this pass.
#[allow(unused_variables)]
pub trait Pass {
    /// Returns the name of this pass.
    fn name(&self) -> Cow<str>;

    /// Optimize at program level.
    fn optimize_program(&mut self, env: &mut OptimizeEnvironment, program: &mut Program);
}

pub trait FunctionOptimizer {
    /// Returns the name of this pass.
    fn name(&self) -> Cow<str>;

    /// Optimize a single function.
    fn optimize_func(&mut self, env: &mut OptimizeEnvironment, func: &mut TacFunc);
}

struct FunctionOptimizerPass<F>(F);
impl<F> Pass for FunctionOptimizerPass<F>
where
    F: FunctionOptimizer,
{
    fn name(&self) -> Cow<str> {
        self.0.name()
    }

    fn optimize_program(&mut self, env: &mut OptimizeEnvironment, program: &mut Program) {
        for func in program.functions.values_mut() {
            self.0.optimize_func(env, func);
        }
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

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            env: OptimizeEnvironment {
                data: AnyMap::new(),
            },
            passes: vec![],
        }
    }

    pub fn add_pass(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass)
    }

    pub fn optimize(mut self, program: &mut Program) {
        for pass in &mut self.passes {
            pass.optimize_program(&mut self.env, program)
        }
    }
}
