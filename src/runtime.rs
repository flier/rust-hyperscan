use std::ptr;
use std::ops::Deref;

use raw::*;
use common::Error;

pub trait Scratch : Clone {
    fn size(&self) -> Result<usize, Error>;

    fn realloc(&mut self, db: *const hs_database_t) -> Result<&Self, Error>;
}

pub struct RawScratch {
    s: *mut hs_scratch_t,
}

impl RawScratch {
    pub fn alloc(db: *const hs_database_t) -> Result<RawScratch, Error> {
        let mut s: *mut hs_scratch_t = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_alloc_scratch(db, &mut s));
        }

        Result::Ok(RawScratch { s: s })
    }
}

impl Drop for RawScratch {
    fn drop(&mut self) {
        unsafe {
            hs_free_scratch(self.s);
        }
    }
}

impl Clone for RawScratch {
    fn clone(&self) -> Self {
        let mut s: *mut hs_scratch_t = ptr::null_mut();

        unsafe {
            hs_clone_scratch(self.s, &mut s);
        }

        RawScratch { s: s }
    }
}

impl Deref for RawScratch {
    type Target = *mut hs_scratch_t;

    fn deref(&self) -> &*mut hs_scratch_t {
        &self.s
    }
}

impl Scratch for RawScratch {
    fn size(&self) -> Result<usize, Error> {
        let mut size: size_t = 0;

        unsafe {
            check_hs_error!(hs_scratch_size(self.s, &mut size));
        }

        Result::Ok(size as usize)
    }

    fn realloc(&mut self, db: *const hs_database_t) -> Result<&Self, Error> {
        unsafe {
            check_hs_error!(hs_alloc_scratch(db, &mut self.s));
        }

        Result::Ok(self)
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;

    use super::super::*;

    #[test]
    fn test_scratch() {
        let db: BlockDatabase = pattern!{"test"}.build().unwrap();

        assert!(*db != ptr::null_mut());

        let s = RawScratch::alloc(*db).unwrap();

        assert!(*s != ptr::null_mut());

        assert_eq!(s.size().unwrap(), 2385);

        let mut s2 = s.clone();

        assert!(*s2 != ptr::null_mut());

        assert_eq!(s2.size().unwrap(), 2385);

        let db2: VectoredDatabase = pattern!{"foobar"}.build().unwrap();

        assert_eq!(s2.realloc(*db2).unwrap().size().unwrap(), 2406);

        assert_eq!(s.size().unwrap(), 2385);
    }
}
