use std::fmt;
use std::ptr;
use std::error;
use std::string::ToString;
use std::ffi::CStr;

use constants::*;
use raw::*;

/// Error Codes
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    /// The engine completed normally.
    Success,
    /// A parameter passed to this function was invalid.
    Invalid,
    /// A memory allocation failed.
    NoMem,
    /// The engine was terminated by callback.
    ///
    /// This return value indicates that the target buffer was partially scanned,
    /// but that the callback function requested that scanning cease after a match was located.
    ScanTerminated,
    /// The pattern compiler failed with more detail.
    CompilerError(String),
    /// The given database was built for a different version of Hyperscan.
    DbVersionError,
    /// The given database was built for a different platform (i.e., CPU type).
    DbPlatformError,
    /// The given database was built for a different mode of operation.
    /// This error is returned when streaming calls are used with a block or vectored database and vice versa.
    DbModeError,
    /// A parameter passed to this function was not correctly aligned.
    BadAlign,
    /// The memory allocator (either malloc() or the allocator set with hs_set_allocator())
    /// did not correctly return memory suitably aligned for the largest representable data type on this platform.
    BadAlloc,
    /// Unknown error code
    Failed(i32),
}

impl From<i32> for Error {
    fn from(err: i32) -> Error {
        match err {
            HS_SUCCESS => Error::Success,
            HS_INVALID => Error::Invalid,
            HS_NOMEM => Error::NoMem,
            HS_SCAN_TERMINATED => Error::ScanTerminated,
            // HS_COMPILER_ERROR => Error::CompilerError,
            HS_DB_VERSION_ERROR => Error::DbVersionError,
            HS_DB_PLATFORM_ERROR => Error::DbPlatformError,
            HS_DB_MODE_ERROR => Error::DbModeError,
            HS_BAD_ALIGN => Error::BadAlign,
            HS_BAD_ALLOC => Error::BadAlloc,
            _ => Error::Failed(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}", error::Error::description(self).to_string()));

        match *self {
            Error::CompilerError(ref reason) => try!(write!(f, " {}", reason)),
            Error::Failed(ref code) => try!(write!(f, " Code: {}", code)),
            _ => {}
        }

        Ok(())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Success => "The engine completed normally.",
            Error::Invalid => "A parameter passed to this function was invalid.",
            Error::NoMem => "A memory allocation failed.",
            Error::ScanTerminated => "The engine was terminated by callback.",
            Error::CompilerError(..) => "The pattern compiler failed.",
            Error::DbVersionError => {
                "The given database was built for a different version of Hyperscan."
            }
            Error::DbPlatformError => "The given database was built for a different platform.",
            Error::DbModeError => "The given database was built for a different mode of operation.",
            Error::BadAlign => "A parameter passed to this function was not correctly aligned.",
            Error::BadAlloc => {
                "The memory allocator did not correctly return memory suitably aligned."
            }
            Error::Failed(..) => "Internal operation failed.",
        }
    }
}

#[macro_export]
macro_rules! check_hs_error {
    ($expr:expr) => (if $expr != $crate::HS_SUCCESS {
        return ::std::result::Result::Err(::std::convert::From::from($expr));
    })
}

#[macro_export]
macro_rules! assert_hs_error {
    ($expr:expr) => (if $expr != $crate::HS_SUCCESS {
        panic!("panic, err={}", $expr);
    })
}

pub trait CompileError : ToString {
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

#[macro_export]
macro_rules! check_compile_error {
    ($expr:expr, $err:ident) => {
        if $crate::HS_SUCCESS != $expr {
            return match $expr {
                $crate::HS_COMPILER_ERROR => {
                    let msg = $crate::errors::RawCompileError($err);

                    ::std::result::Result::Err($crate::errors::Error::CompilerError(msg.to_string()))
                },
                _ =>
                    ::std::result::Result::Err(::std::convert::From::from($expr)),
            }
        }
    }
}
