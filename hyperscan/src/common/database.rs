use core::marker::PhantomData;
use core::ptr;
use std::ffi::CStr;

use failure::Error;
use foreign_types::{foreign_type, ForeignTypeRef};

use crate::common::{Block, Mode, Streaming, Vectored};
use crate::errors::{AsResult, ErrorKind::*};

foreign_type! {
    /// A compiled pattern database that can then be used to scan data.
    pub type Database<T>: Send + Sync {
        type CType = ffi::hs_database_t;
        type PhantomData = PhantomData<T>;

        fn drop = drop_database;
    }
}

unsafe fn drop_database(db: *mut ffi::hs_database_t) {
    ffi::hs_free_database(db).ok().unwrap();
}

/// Block scan (non-streaming) database.
pub type BlockDatabase = Database<Block>;
/// Streaming database.
pub type StreamingDatabase = Database<Streaming>;
/// Vectored scanning database.
pub type VectoredDatabase = Database<Vectored>;

impl<T> Database<T>
where
    T: Mode,
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
    pub fn size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;

        unsafe { ffi::hs_database_size(self.as_ptr(), &mut size).ok().map(|_| size) }
    }

    /// Utility function providing information about a database.
    pub fn info(&self) -> Result<String, Error> {
        let mut p = ptr::null_mut();

        unsafe {
            ffi::hs_database_info(self.as_ptr(), &mut p).ok().and_then(|_| {
                let info = CStr::from_ptr(p)
                    .to_str()
                    .map(|s| s.to_owned())
                    .map_err(|_| Invalid.into());

                if !p.is_null() {
                    libc::free(p as *mut _)
                }

                info
            })
        }
    }
}

#[cfg(test)]
pub mod tests {
    use regex::Regex;

    use crate::common::Mode;
    use crate::common::*;
    use crate::compile::PlatformInfo;

    const DATABASE_SIZE: usize = 872;

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

    pub fn validate_serialized_database(data: &SerializedDatabase) {
        assert!(data.size().unwrap() >= DATABASE_SIZE);

        let db_info = data.info().unwrap();

        validate_database_info(&db_info);
    }

    #[test]
    pub fn test_platform() {
        assert!(PlatformInfo::is_valid().is_ok())
    }

    #[test]
    fn test_database() {
        let _ = pretty_env_logger::try_init();

        let db = BlockDatabase::compile("test", 0, None).unwrap();

        validate_database(&db);

        assert_eq!(db.name(), "Block");
    }
}
