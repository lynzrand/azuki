pub mod tac;

pub(crate) unsafe fn unbox<T>(ptr: *mut T) -> T {
    *Box::from_raw(ptr)
}

pub(crate) unsafe fn dropbox<T>(ptr: *mut T) {
    drop(Box::from_raw(ptr))
}
