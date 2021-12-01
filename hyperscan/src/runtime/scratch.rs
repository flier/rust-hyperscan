use std::mem::MaybeUninit;
use std::ptr::NonNull;

use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::{common::DatabaseRef, error::AsResult, ffi, Result};

foreign_type! {
    /// A large enough region of scratch space to support a given database.
    pub unsafe type Scratch: Send {
        type CType = ffi::hs_scratch_t;

        fn drop = free_scratch;
        fn clone = clone_scratch;
    }
}

unsafe fn free_scratch(s: *mut ffi::hs_scratch_t) {
    ffi::hs_free_scratch(s).expect("free scratch");
}

unsafe fn clone_scratch(s: *mut ffi::hs_scratch_t) -> *mut ffi::hs_scratch_t {
    let mut p = MaybeUninit::uninit();
    ffi::hs_clone_scratch(s, p.as_mut_ptr()).expect("clone scratch");
    p.assume_init()
}

impl Scratch {
    /// Allocate a "scratch" space for use by Hyperscan.
    ///
    /// This is required for runtime use, and one scratch space per thread,
    /// or concurrent caller, is required.
    ///
    unsafe fn alloc<T>(db: &DatabaseRef<T>) -> Result<Scratch> {
        let mut s = MaybeUninit::zeroed();
        ffi::hs_alloc_scratch(db.as_ptr(), s.as_mut_ptr()).map(|_| Scratch::from_ptr(s.assume_init()))
    }

    /// Reallocate a "scratch" space for use by Hyperscan.
    unsafe fn realloc<T>(&mut self, db: &DatabaseRef<T>) -> Result<()> {
        let mut p = self.as_ptr();

        ffi::hs_alloc_scratch(db.as_ptr(), &mut p).map(|_| {
            self.0 = NonNull::new_unchecked(p);
        })
    }
}

impl ScratchRef {
    /// Provides the size of the given scratch space.
    pub fn size(&self) -> Result<usize> {
        let mut size = MaybeUninit::uninit();

        unsafe { ffi::hs_scratch_size(self.as_ptr(), size.as_mut_ptr()).map(|_| size.assume_init()) }
    }
}

impl<T> DatabaseRef<T> {
    /// Allocate a "scratch" space for use by Hyperscan.
    pub fn alloc_scratch(&self) -> Result<Scratch> {
        unsafe { Scratch::alloc(self) }
    }

    /// Reallocate a "scratch" space for use by Hyperscan.
    pub fn realloc_scratch<'a>(&'a self, s: &'a mut Scratch) -> Result<&'a mut Scratch> {
        unsafe { s.realloc(self) }.map(|_| s)
    }

    /// Allocate a "scratch" space for use by Hyperscan.
    #[deprecated = "use `alloc_scratch` instead"]
    pub fn alloc(&self) -> Result<Scratch> {
        unsafe { Scratch::alloc(self) }
    }

    /// Reallocate a "scratch" space for use by Hyperscan.
    #[deprecated = "use `realloc_scratch` instead"]
    pub fn realloc(&self, s: &mut Scratch) -> Result<()> {
        unsafe { s.realloc(self) }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::prelude::*;

    const SCRATCH_SIZE: usize = 2000;

    #[test]
    fn test_scratch() {
        let db: BlockDatabase = "test".parse().unwrap();

        let s = db.alloc_scratch().unwrap();

        assert!(s.size().unwrap() > SCRATCH_SIZE);

        let mut s2 = s.clone();

        assert!(s2.size().unwrap() > SCRATCH_SIZE);

        let db2: VectoredDatabase = "foobar".parse().unwrap();

        db2.realloc_scratch(&mut s2).unwrap();

        assert!(s2.size().unwrap() > s.size().unwrap());
    }
}
