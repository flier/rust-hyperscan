use std::ffi::CStr;
use std::ptr;

use anyhow::Result;
use foreign_types::{foreign_type, ForeignTypeRef};

use crate::chimera::{errors::AsResult, ffi};

/// Utility function for identifying this release version.
pub fn version() -> &'static CStr {
    unsafe { CStr::from_ptr(ffi::ch_version()) }
}

foreign_type! {
    /// A compiled pattern database that can then be used to scan data.
    pub unsafe type Database: Send + Sync {
        type CType = ffi::ch_database_t;

        fn drop = drop_database;
    }
}

unsafe fn drop_database(db: *mut ffi::ch_database_t) {
    ffi::ch_free_database(db).expect("drop database");
}

impl DatabaseRef {
    /// Returns the size of the given database.
    pub fn size(&self) -> Result<usize> {
        let mut size: usize = 0;

        unsafe { ffi::ch_database_size(self.as_ptr(), &mut size).map(|_| size) }
    }

    /// Utility function providing information about a database.
    pub fn info(&self) -> Result<String> {
        let mut p = ptr::null_mut();

        unsafe {
            ffi::ch_database_info(self.as_ptr(), &mut p).and_then(|_| {
                let info = CStr::from_ptr(p).to_str()?.to_owned();

                libc::free(p as *mut _);

                Ok(info)
            })
        }
    }
}
