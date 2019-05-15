use core::fmt;
use std::ffi::CStr;

use failure::AsFail;
use foreign_types::{foreign_type, ForeignType};

use crate::errors::{AsResult, HsError};

pub trait AsCompileResult {
    type Output;
    type Error: AsFail;

    fn ok_or(self, err: *mut ffi::hs_compile_error_t) -> Result<Self::Output, Self::Error>;
}

impl AsCompileResult for ffi::hs_error_t {
    type Output = ();
    type Error = failure::Error;

    fn ok_or(self, err: *mut ffi::hs_compile_error_t) -> Result<Self::Output, Self::Error> {
        if self == ffi::HS_SUCCESS as ffi::hs_error_t {
            Ok(())
        } else if self == ffi::HS_COMPILER_ERROR && !err.is_null() {
            Err(HsError::CompileError(unsafe { Error::from_ptr(err) }).into())
        } else {
            Err(HsError::from(self).into())
        }
    }
}

foreign_type! {
    /// Providing details of the compile error condition.
    pub type Error: Send + Sync {
        type CType = ffi::hs_compile_error_t;

        fn drop = free_compile_error;
    }
}

unsafe fn free_compile_error(err: *mut ffi::hs_compile_error_t) {
    ffi::hs_free_compile_error(err).ok().unwrap();
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error")
            .field("message", &self.message())
            .field("expression", &self.expression())
            .finish()
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl Error {
    unsafe fn as_ref(&self) -> &ffi::hs_compile_error_t {
        self.as_ptr().as_ref().unwrap()
    }

    pub fn message(&self) -> &str {
        unsafe { CStr::from_ptr(self.as_ref().message).to_str().unwrap() }
    }

    pub fn expression(&self) -> usize {
        unsafe { self.as_ref().expression as usize }
    }
}
