use std::{
    borrow::BorrowMut,
    cell::RefCell,
    ops::{Deref, DerefMut},
};

use azuki_tac::{Branch, FunctionCall, Inst, TacFunc};

use crate::Frame;

/// Trait for collecting data about the code running.
pub trait Inspector {
    /// Called before every instruction is runned.
    fn before_inst(&mut self, inst: &Inst, frame: &Frame);
    /// Called before every instruction is runned.
    fn before_branch(&mut self, inst: &Branch, frame: &Frame);
    /// Called before every function call.
    fn before_call(&mut self, params: &[i64], func: &TacFunc);
    /// Called after every function is returned
    fn before_ret(&mut self, frame: &Frame);
}

impl<R, T> Inspector for R
where
    R: DerefMut<Target = T>,
    T: Inspector,
{
    fn before_inst(&mut self, inst: &Inst, frame: &Frame) {
        self.borrow_mut().before_inst(inst, frame)
    }

    fn before_branch(&mut self, inst: &Branch, frame: &Frame) {
        self.borrow_mut().before_branch(inst, frame)
    }

    fn before_call(&mut self, params: &[i64], func: &TacFunc) {
        self.borrow_mut().before_call(params, func)
    }

    fn before_ret(&mut self, frame: &Frame) {
        self.borrow_mut().before_ret(frame)
    }
}
