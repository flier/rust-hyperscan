use core::fmt;
use std::ffi::CStr;

use foreign_types::{foreign_type, ForeignTypeRef};

use crate::errors::AsResult;

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

macro_rules! check_compile_error {
    ($expr:expr, $err:ident) => {
        if $crate::HS_SUCCESS != $expr {
            return match $expr {
                $crate::HS_COMPILER_ERROR => {
                    let msg = $crate::compile::Error::from_ptr($err);

                    Err($crate::errors::HsError::CompileError(msg).into())
                }
                _ => Err($crate::errors::HsError::from($expr).into()),
            };
        }
    };
}
