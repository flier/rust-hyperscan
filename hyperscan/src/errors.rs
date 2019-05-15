use core::fmt;
use std::ffi::CStr;

use failure::{AsFail, Error, Fail};
use foreign_types::{foreign_type, ForeignTypeRef};

/// Error Codes
#[derive(Debug, PartialEq, Fail)]
pub enum HsError {
    #[fail(display = "A parameter passed to this function was invalid.")]
    Invalid,

    #[fail(display = "A memory allocation failed.")]
    NoMem,

    #[fail(display = "The engine was terminated by callback.")]
    ScanTerminated,

    #[fail(display = "The pattern compiler failed with more detail, {}.", _0)]
    CompilerError(CompilerError),

    #[fail(display = "The given database was built for a different version of Hyperscan.")]
    DbVersionError,

    #[fail(display = "The given database was built for a different platform (i.e., CPU type).")]
    DbPlatformError,

    #[fail(display = "The given database was built for a different mode of operation.")]
    DbModeError,

    #[fail(display = "A parameter passed to this function was not correctly aligned.")]
    BadAlign,

    #[fail(display = "The memory allocator did not correctly return memory suitably aligned.")]
    BadAlloc,

    #[fail(display = "The scratch region was already in use.")]
    ScratchInUse,

    #[fail(display = "Unsupported CPU architecture.")]
    ArchError,

    #[fail(display = "Provided buffer was too small.")]
    InsufficientSpace,

    #[fail(display = "Unexpected internal error.")]
    UnknownError,

    #[fail(display = "Unknown error code: {}", _0)]
    Code(ffi::hs_error_t),
}

impl From<ffi::hs_error_t> for HsError {
    fn from(err: ffi::hs_error_t) -> HsError {
        use HsError::*;

        match err {
            ffi::HS_INVALID => Invalid,
            ffi::HS_NOMEM => NoMem,
            ffi::HS_SCAN_TERMINATED => ScanTerminated,
            // ffi::HS_COMPILER_ERROR => HsError::CompilerError,
            ffi::HS_DB_VERSION_ERROR => DbVersionError,
            ffi::HS_DB_PLATFORM_ERROR => DbPlatformError,
            ffi::HS_DB_MODE_ERROR => DbModeError,
            ffi::HS_BAD_ALIGN => BadAlign,
            ffi::HS_BAD_ALLOC => BadAlloc,
            ffi::HS_SCRATCH_IN_USE => ScratchInUse,
            ffi::HS_ARCH_ERROR => ArchError,
            ffi::HS_INSUFFICIENT_SPACE => InsufficientSpace,
            ffi::HS_UNKNOWN_ERROR => UnknownError,
            _ => Code(err),
        }
    }
}

pub trait AsResult {
    type Output;
    type Error: AsFail;

    fn ok(self) -> Result<Self::Output, Self::Error>;
}

impl AsResult for ffi::hs_error_t {
    type Output = Self;
    type Error = Error;

    fn ok(self) -> Result<Self::Output, Self::Error> {
        if self == ffi::HS_SUCCESS as ffi::hs_error_t {
            Ok(self)
        } else {
            Err(HsError::from(self).into())
        }
    }
}

foreign_type! {
    /// Providing details of the compile error condition.
    pub type CompilerError: Send + Sync {
        type CType = ffi::hs_compile_error_t;

        fn drop = free_compile_error;
    }
}

unsafe fn free_compile_error(err: *mut ffi::hs_compile_error_t) {
    ffi::hs_free_compile_error(err).ok().unwrap();
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl fmt::Debug for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CompilerError")
            .field("message", &self.message())
            .field("expression", &self.expression())
            .finish()
    }
}

impl PartialEq for CompilerError {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl CompilerError {
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
                    let msg = $crate::errors::CompilerError::from_ptr($err);

                    Err($crate::errors::HsError::CompilerError(msg).into())
                }
                _ => Err($crate::errors::HsError::from($expr).into()),
            };
        }
    };
}