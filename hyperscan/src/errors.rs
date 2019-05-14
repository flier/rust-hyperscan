use std::ffi::CStr;

use failure::{AsFail, Error, Fail};
use foreign_types::{foreign_type, ForeignTypeRef};

use crate::constants::*;

/// Error Codes
#[derive(Debug, PartialEq, Fail)]
pub enum ErrorKind {
    #[fail(display = "A parameter passed to this function was invalid.")]
    Invalid,

    #[fail(display = "A memory allocation failed.")]
    NoMem,

    /// This return value indicates that the target buffer was partially scanned,
    /// but that the callback function requested that scanning cease after a match was located.
    #[fail(display = "The engine was terminated by callback.")]
    ScanTerminated,

    #[fail(display = "The pattern compiler failed with more detail, {}.", _0)]
    CompilerError(String),

    #[fail(display = "The given database was built for a different version of Hyperscan.")]
    DbVersionError,

    #[fail(display = "The given database was built for a different platform (i.e., CPU type).")]
    DbPlatformError,

    /// This error is returned when streaming calls are used
    /// with a block or vectored database and vice versa.
    #[fail(display = "The given database was built for a different mode of operation.")]
    DbModeError,

    #[fail(display = "A parameter passed to this function was not correctly aligned.")]
    BadAlign,

    /// The memory allocator (either malloc() or the allocator set with hs_set_allocator())
    /// did not correctly return memory suitably aligned
    /// for the largest representable data type on this platform.
    #[fail(display = "The memory allocator did not correctly return memory suitably aligned.")]
    BadAlloc,

    #[fail(display = "Unknown error code")]
    Code(i32),
}

impl From<i32> for ErrorKind {
    fn from(err: i32) -> ErrorKind {
        match err {
            HS_SUCCESS => unreachable!(),
            HS_INVALID => ErrorKind::Invalid,
            HS_NOMEM => ErrorKind::NoMem,
            HS_SCAN_TERMINATED => ErrorKind::ScanTerminated,
            // HS_COMPILER_ERROR => ErrorKind::CompilerError,
            HS_DB_VERSION_ERROR => ErrorKind::DbVersionError,
            HS_DB_PLATFORM_ERROR => ErrorKind::DbPlatformError,
            HS_DB_MODE_ERROR => ErrorKind::DbModeError,
            HS_BAD_ALIGN => ErrorKind::BadAlign,
            HS_BAD_ALLOC => ErrorKind::BadAlloc,
            _ => ErrorKind::Code(err),
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
        if self == HS_SUCCESS {
            Ok(self)
        } else {
            Err(ErrorKind::from(self).into())
        }
    }
}

foreign_type! {
    /// Providing details of the compile error condition.
    pub type CompileError {
        type CType = ffi::hs_compile_error_t;

        fn drop = free_compile_error;
    }
}

unsafe fn free_compile_error(err: *mut ffi::hs_compile_error_t) {
    ffi::hs_free_compile_error(err).ok().unwrap();
}

impl CompileError {
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
                    let msg = $crate::errors::CompileError::from_ptr($err);

                    Err($crate::errors::ErrorKind::CompilerError(msg.message().to_owned()).into())
                }
                _ => Err($crate::errors::ErrorKind::from($expr).into()),
            };
        }
    };
}
