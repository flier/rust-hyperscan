use std::mem::MaybeUninit;

use anyhow::Result;
use foreign_types::{foreign_type, ForeignTypeRef};

use crate::chimera::{errors::AsResult, ffi};

foreign_type! {
    /// A large enough region of scratch space to support a given database.
    pub unsafe type Scratch {
        type CType = ffi::ch_scratch_t;

        fn drop = free_scratch;
        fn clone = clone_scratch;
    }
}

/// Free a scratch block previously allocated by `ch_alloc_scratch()` or `ch_clone_scratch()`.
unsafe fn free_scratch(s: *mut ffi::ch_scratch_t) {
    ffi::ch_free_scratch(s).expect("free scratch");
}

/// Allocate a scratch space that is a clone of an existing scratch space.
unsafe fn clone_scratch(s: *mut ffi::ch_scratch_t) -> *mut ffi::ch_scratch_t {
    let mut p = MaybeUninit::uninit();
    ffi::ch_clone_scratch(s, p.as_mut_ptr()).expect("clone scratch");
    p.assume_init()
}

impl ScratchRef {
    /// Provides the size of the given scratch space.
    pub fn size(&self) -> Result<usize> {
        let mut size = MaybeUninit::uninit();

        unsafe { ffi::ch_scratch_size(self.as_ptr(), size.as_mut_ptr()).map(|_| size.assume_init()) }
    }
}
