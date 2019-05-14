use std::ffi::CStr;
use std::fmt;
use std::ptr;
use std::string::ToString;

use failure::Fail;

use crate::constants::*;
use crate::ffi::*;

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

macro_rules! check_hs_error {
    ($expr:expr) => {
        if $expr != $crate::HS_SUCCESS {
            return ::std::result::Result::Err($crate::errors::ErrorKind::from($expr).into());
        }
    };
}

macro_rules! assert_hs_error {
    ($expr:expr) => {
        if $expr != $crate::HS_SUCCESS {
            panic!("panic, err={}", $expr);
        }
    };
}

pub trait CompileError: ToString {
    fn expression(&self) -> usize;
}

pub type RawCompileErrorPtr = *mut hs_compile_error_t;

/// Providing details of the compile error condition.
pub struct RawCompileError(pub RawCompileErrorPtr);

impl fmt::Debug for RawCompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawCompileError({:p})", self.0)
    }
}

impl CompileError for RawCompileError {
    #[inline]
    fn expression(&self) -> usize {
        unsafe { (*self.0).expression as usize }
    }
}

impl ToString for RawCompileError {
    #[inline]
    fn to_string(&self) -> String {
        unsafe { String::from(CStr::from_ptr((*self.0).message).to_str().unwrap()) }
    }
}

impl Drop for RawCompileError {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if self.0 != ptr::null_mut() {
                assert_hs_error!(hs_free_compile_error(self.0));
            }
        }
    }
}

macro_rules! check_compile_error {
    ($expr:expr, $err:ident) => {
        if $crate::HS_SUCCESS != $expr {
            return match $expr {
                $crate::HS_COMPILER_ERROR => {
                    let msg = $crate::errors::RawCompileError($err);

                    Err($crate::errors::ErrorKind::CompilerError(msg.to_string()).into())
                }
                _ => Err($crate::errors::ErrorKind::from($expr).into()),
            };
        }
    };
}
