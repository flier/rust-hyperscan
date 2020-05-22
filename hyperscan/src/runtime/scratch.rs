use core::ptr::{null_mut, NonNull};

use failure::Error;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::common::{Database, DatabaseRef};
use crate::errors::AsResult;
use crate::ffi;

foreign_type! {
    /// A large enough region of scratch space to support a given database.
    pub unsafe type Scratch {
        type CType = ffi::hs_scratch_t;

        fn drop = free_scratch;
        fn clone = clone_scratch;
    }
}

unsafe fn free_scratch(s: *mut ffi::hs_scratch_t) {
    ffi::hs_free_scratch(s).expect("free scratch");
}

unsafe fn clone_scratch(s: *mut ffi::hs_scratch_t) -> *mut ffi::hs_scratch_t {
    let mut p = null_mut();
    ffi::hs_clone_scratch(s, &mut p).expect("clone scratch");
    p
}

impl Scratch {
    /// Allocate a "scratch" space for use by Hyperscan.
    ///
    /// This is required for runtime use, and one scratch space per thread,
    /// or concurrent caller, is required.
    ///
    unsafe fn alloc<T>(db: &DatabaseRef<T>) -> Result<Scratch, Error> {
        let mut s = null_mut();

        ffi::hs_alloc_scratch(db.as_ptr(), &mut s).map(|_| Scratch::from_ptr(s))
    }

    /// Reallocate a "scratch" space for use by Hyperscan.
    unsafe fn realloc<T>(&mut self, db: &DatabaseRef<T>) -> Result<(), Error> {
        let mut p = self.as_ptr();

        ffi::hs_alloc_scratch(db.as_ptr(), &mut p).map(|_| {
            self.0 = NonNull::new_unchecked(p);
        })
    }
}

impl ScratchRef {
    /// Provides the size of the given scratch space.
    pub fn size(&self) -> Result<usize, Error> {
        let mut size = 0;

        unsafe { ffi::hs_scratch_size(self.as_ptr(), &mut size).map(|_| size) }
    }
}

impl<T> Database<T> {
    /// Allocate a "scratch" space for use by Hyperscan.
    pub fn alloc(&self) -> Result<Scratch, Error> {
        unsafe { Scratch::alloc(self) }
    }

    /// Reallocate a "scratch" space for use by Hyperscan.
    pub fn realloc(&self, s: &mut Scratch) -> Result<(), Error> {
        unsafe { s.realloc(self) }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::prelude::*;

    const SCRATCH_SIZE: usize = 2000;

    #[test]
    fn test_scratch() {
        let _ = pretty_env_logger::try_init();

        let db: BlockDatabase = pattern! {"test"}.build().unwrap();

        let s = db.alloc().unwrap();

        assert!(s.size().unwrap() > SCRATCH_SIZE);

        let mut s2 = s.clone();

        assert!(s2.size().unwrap() > SCRATCH_SIZE);

        let db2: VectoredDatabase = pattern! {"foobar"}.build().unwrap();

        db2.realloc(&mut s2).unwrap();

        assert!(s2.size().unwrap() > s.size().unwrap());
    }
}
