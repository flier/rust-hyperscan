use core::ptr::{null_mut, NonNull};

use failure::Error;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::api::*;
use crate::database::{Database, DatabaseRef};
use crate::errors::AsResult;

foreign_type! {
    /// A large enough region of scratch space to support a given database.
    pub type Scratch {
        type CType = ffi::hs_scratch_t;

        fn drop = free_scratch;
        fn clone = clone_scratch;
    }
}

unsafe fn free_scratch(s: *mut ffi::hs_scratch_t) {
    ffi::hs_free_scratch(s).ok().unwrap();
}

unsafe fn clone_scratch(s: *mut ffi::hs_scratch_t) -> *mut ffi::hs_scratch_t {
    let mut p = null_mut();
    ffi::hs_clone_scratch(s, &mut p).ok().unwrap();
    p
}

impl Scratch {
    /// Allocate a "scratch" space for use by Hyperscan.
    ///
    /// This is required for runtime use, and one scratch space per thread,
    /// or concurrent caller, is required.
    ///
    unsafe fn alloc<T: Mode>(db: &DatabaseRef<T>) -> Result<Scratch, Error> {
        let mut s = null_mut();

        ffi::hs_alloc_scratch(db.as_ptr(), &mut s)
            .ok()
            .map(|_| Scratch::from_ptr(s))
    }

    /// Reallocate a "scratch" space for use by Hyperscan.
    unsafe fn realloc<T: Mode>(&mut self, db: &DatabaseRef<T>) -> Result<(), Error> {
        let mut p = self.as_ptr();

        ffi::hs_alloc_scratch(db.as_ptr(), &mut p).ok().map(|_| {
            self.0 = NonNull::new_unchecked(p);
        })
    }
}

impl ScratchRef {
    /// Provides the size of the given scratch space.
    pub fn size(&self) -> Result<usize, Error> {
        let mut size = 0;

        unsafe { ffi::hs_scratch_size(self.as_ptr(), &mut size).ok().map(|_| size) }
    }
}

impl<T: Mode> Database<T> {
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
    extern crate pretty_env_logger;

    use crate::constants::*;
    use crate::database::*;
    use crate::errors::ErrorKind;
    use crate::runtime::*;

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

    #[test]
    fn test_block_scan() {
        let _ = pretty_env_logger::try_init();

        let db: BlockDatabase = pattern! {"test", flags => HS_FLAG_CASELESS|HS_FLAG_SOM_LEFTMOST}
            .build()
            .unwrap();
        let s = db.alloc().unwrap();

        db.scan::<_, ()>("foo test bar", &s, None, None).unwrap();

        fn callback(id: u32, from: u64, to: u64, flags: u32, _: &BlockDatabase) -> u32 {
            assert_eq!(id, 0);
            assert_eq!(from, 4);
            assert_eq!(to, 8);
            assert_eq!(flags, 0);

            1
        };

        assert_eq!(
            db.scan("foo test bar".as_bytes(), &s, Some(callback), Some(&db))
                .err()
                .unwrap()
                .downcast_ref::<ErrorKind>(),
            Some(&ErrorKind::ScanTerminated)
        );
    }

    #[test]
    fn test_vectored_scan() {
        let _ = pretty_env_logger::try_init();

        let db: VectoredDatabase = pattern! {"test", flags => HS_FLAG_CASELESS|HS_FLAG_SOM_LEFTMOST}
            .build()
            .unwrap();
        let s = db.alloc().unwrap();

        let data = vec!["foo".as_bytes(), "test".as_bytes(), "bar".as_bytes()];

        db.scan::<()>(&data, &s, None, None).unwrap();

        fn callback(id: u32, from: u64, to: u64, flags: u32, _: &VectoredDatabase) -> u32 {
            assert_eq!(id, 0);
            assert_eq!(from, 3);
            assert_eq!(to, 7);
            assert_eq!(flags, 0);

            1
        };

        let data = vec!["foo".as_bytes(), "test".as_bytes(), "bar".as_bytes()];

        assert_eq!(
            db.scan::<_>(&data, &s, Some(callback), Some(&db))
                .err()
                .unwrap()
                .downcast_ref::<ErrorKind>(),
            Some(&ErrorKind::ScanTerminated)
        );
    }

    #[test]
    fn test_streaming_scan() {
        let _ = pretty_env_logger::try_init();

        let db: StreamingDatabase = pattern! {"test", flags => HS_FLAG_CASELESS}.build().unwrap();

        let s = db.alloc().unwrap();
        let st = db.open_stream().unwrap();

        let data = vec!["foo", "test", "bar"];

        fn callback(id: u32, from: u64, to: u64, flags: u32, _: &StreamingDatabase) -> u32 {
            assert_eq!(id, 0);
            assert_eq!(from, 0);
            assert_eq!(to, 7);
            assert_eq!(flags, 0);

            0
        }

        for d in data {
            st.scan(d, &s, Some(callback), Some(&db)).unwrap();
        }

        st.close(&s, Some(callback), Some(&db)).unwrap();
    }
}
