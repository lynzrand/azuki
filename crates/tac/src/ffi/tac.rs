//! Exported C API and related stuff.
#![allow(clippy::missing_safety_doc)]

use crate::{Inst, TacFunc, Ty};
use std::ffi::*;

use super::{dropbox, unbox};

#[no_mangle]
pub unsafe extern "C" fn AzukiTacFunc_NewUntyped(name: *const i8) -> *mut TacFunc {
    let name = CStr::from_ptr(name).to_string_lossy();
    Box::into_raw(Box::new(TacFunc::new_untyped(name.into())))
}

#[no_mangle]
pub unsafe extern "C" fn AzukiTacFunc_New(name: *const i8, ty: *mut Ty) -> *mut TacFunc {
    let name = CStr::from_ptr(name).to_string_lossy();
    let ty = unbox(ty);
    Box::into_raw(Box::new(TacFunc::new(name.into(), ty)))
}

#[no_mangle]
pub unsafe extern "C" fn AzukiTacFunc_AddInst(
    func: *mut TacFunc,
    inst: *mut Inst,
    bb: usize,
) -> u64 {
    let inst = unbox(inst);
    func.as_mut().unwrap().tac_new(inst, bb).to_bits()
}

#[no_mangle]
pub unsafe extern "C" fn AzukiTacFunc_Destructor(func: *mut TacFunc) {
    dropbox(func)
}
