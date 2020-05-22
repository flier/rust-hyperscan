use std::ffi::CStr;
use std::fmt;

use foreign_types::{foreign_type, ForeignType};

use crate::errors::{AsResult, HsError};
use crate::ffi;

pub trait AsCompileResult {
    type Output;
    type Error: fmt::Display;

    fn ok_or(self, err: *mut ffi::hs_compile_error_t) -> Result<Self::Output, Self::Error>;
}

impl AsCompileResult for ffi::hs_error_t {
    type Output = ();
    type Error = anyhow::Error;

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
    pub unsafe type Error: Send + Sync {
        type CType = ffi::hs_compile_error_t;

        fn drop = free_compile_error;
    }
}

unsafe fn free_compile_error(err: *mut ffi::hs_compile_error_t) {
    ffi::hs_free_compile_error(err).expect("free compile error");
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

    /// A human-readable error message describing the error.
    pub fn message(&self) -> &str {
        unsafe { CStr::from_ptr(self.as_ref().message).to_str().unwrap() }
    }

    /// The zero-based number of the expression that caused the error (if this can be determined).
    pub fn expression(&self) -> Option<usize> {
        let n = unsafe { self.as_ref().expression };

        if n < 0 {
            None
        } else {
            Some(n as usize)
        }
    }
}
