use std::fmt;
use std::error;

use constants::*;

#[derive(Debug)]
pub enum Error {
    Success,
    Failed(i32),
    Invalid,
    NoMem,
    ScanTerminated,
    CompilerError(String),
    DbVersionError,
    DbPlatformError,
    DbModeError,
    BadAlign,
    BadAlloc,
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
        write!(f, "{}", error::Error::description(self).to_string())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Success => "The engine completed normally.",
            Error::Failed(..) => "Failed.",
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
        }
    }
}

#[macro_export]
macro_rules! check_hs_error {
    ($expr:expr) => (if $expr != $crate::constants::HS_SUCCESS {
        return $crate::std::result::Result::Err($crate::std::convert::From::from($expr));
    })
}

#[macro_export]
macro_rules! check_compile_error {
    ($expr:expr, $err:ident) => {
        if $crate::constants::HS_SUCCESS != $expr {
            return match $expr {
                $crate::constants::HS_COMPILER_ERROR => {
                    let msg = $crate::std::ffi::CString::from_raw((*$err).message).into_string().unwrap();

                    $crate::std::result::Result::Err($crate::errors::Error::CompilerError(msg))
                },
                _ =>
                    $crate::std::result::Result::Err($crate::std::convert::From::from($expr)),
            }
        }
    }
}
