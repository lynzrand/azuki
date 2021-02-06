//! Exported C API and related stuff.
#![allow(clippy::missing_safety_doc)]

use crate::{Inst, TacFunc};
use std::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn azuki_tac_func_new_untyped(name: *const i8) -> *mut TacFunc {
    let name = CStr::from_ptr(name).to_string_lossy();
    Box::into_raw(Box::new(TacFunc::new_undefined_type(name.into())))
}

#[no_mangle]
pub unsafe extern "C" fn azuki_tac_func_add_inst(
    func: *mut TacFunc,
    inst: *mut Inst,
    bb: usize,
) -> u64 {
    let inst = *Box::from_raw(inst);
    func.as_mut().unwrap().tac_new(inst, bb).to_bits()
}

#[no_mangle]
pub unsafe extern "C" fn azuki_tac_func_drop(func: *mut TacFunc) {
    drop(Box::from_raw(func))
}
