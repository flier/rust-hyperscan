use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

use foreign_types::{foreign_type, ForeignTypeRef};

use crate::{
    common::{Block, Mode, Streaming, Vectored},
    error::AsResult,
    ffi, Result,
};

foreign_type! {
    /// A compiled pattern database that can then be used to scan data.
    pub unsafe type Database<T>: Send + Sync {
        type CType = ffi::hs_database_t;
        type PhantomData = PhantomData<T>;

        fn drop = drop_database;
    }
}

unsafe fn drop_database(db: *mut ffi::hs_database_t) {
    ffi::hs_free_database(db).expect("drop database");
}

/// Block scan (non-streaming) database.
pub type BlockDatabase = Database<Block>;
/// Streaming database.
pub type StreamingDatabase = Database<Streaming>;
/// Vectored scanning database.
pub type VectoredDatabase = Database<Vectored>;

impl<T> DatabaseRef<T>
where
    T: Mode + 'static,
{
    /// Provides the id of compiled mode of the given database.
    pub fn id(&self) -> u32 {
        T::ID
    }

    /// Provides the name of compiled mode of the given database.
    pub fn name(&self) -> &'static str {
        T::NAME
    }
}

impl<T> DatabaseRef<T> {
    /// Provides the size of the given database in bytes.
    pub fn size(&self) -> Result<usize> {
        let mut size = MaybeUninit::uninit();

        unsafe { ffi::hs_database_size(self.as_ptr(), size.as_mut_ptr()).map(|_| size.assume_init()) }
    }

    /// Utility function providing information about a database.
    pub fn info(&self) -> Result<String> {
        let mut p = MaybeUninit::uninit();

        unsafe {
            ffi::hs_database_info(self.as_ptr(), p.as_mut_ptr()).and_then(|_| {
                let p = p.assume_init();
                let info = CStr::from_ptr(p).to_str()?.to_owned();
                libc::free(p as *mut _);
                Ok(info)
            })
        }
    }
}

#[cfg(test)]
pub mod tests {
    use regex::Regex;

    use crate::prelude::*;

    use super::*;

    pub const DATABASE_SIZE: usize = 872;

    pub fn validate_database_info(info: &str) -> (Vec<u8>, Option<String>, Option<String>) {
        if let Some(captures) = Regex::new(r"^Version:\s(\d\.\d\.\d)\sFeatures:\s+(\w+)?\sMode:\s(\w+)$")
            .unwrap()
            .captures(info)
        {
            let version = captures
                .get(1)
                .unwrap()
                .as_str()
                .split('.')
                .flat_map(|s| s.parse())
                .collect();
            let features = captures.get(2).map(|m| m.as_str().to_owned());
            let mode = captures.get(3).map(|m| m.as_str().to_owned());

            (version, features, mode)
        } else {
            panic!("fail to parse database info: {}", info);
        }
    }

    pub fn validate_database_with_size<T: Mode>(db: &DatabaseRef<T>, size: usize) {
        assert!(db.size().unwrap() >= size);

        let db_info = db.info().unwrap();

        validate_database_info(&db_info);
    }

    pub fn validate_database<T: Mode>(db: &DatabaseRef<T>) {
        validate_database_with_size(db, DATABASE_SIZE);
    }

    #[test]
    fn test_database() {
        let db: BlockDatabase = "test".parse().unwrap();

        assert!(db.size().unwrap() > 0);
        assert_eq!(db.name(), "Block");

        let db_info = db.info().unwrap();

        validate_database_info(&db_info);
    }
}
