use std::ffi::CStr;
use std::mem::MaybeUninit;

use foreign_types::{foreign_type, ForeignTypeRef};

use crate::{
    chimera::{error::AsResult, ffi},
    Result,
};

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
        let mut size = MaybeUninit::uninit();

        unsafe { ffi::ch_database_size(self.as_ptr(), size.as_mut_ptr()).map(|_| size.assume_init()) }
    }

    /// Utility function providing information about a database.
    pub fn info(&self) -> Result<String> {
        let mut p = MaybeUninit::uninit();

        unsafe {
            ffi::ch_database_info(self.as_ptr(), p.as_mut_ptr()).and_then(|_| {
                let p = p.assume_init();
                let info = CStr::from_ptr(p).to_str()?.into();
                libc::free(p as *mut _);
                Ok(info)
            })
        }
    }
}
