use std::ffi::CStr;
use std::result::Result as StdResult;

use failure::Error;

use constants::*;
use raw::*;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, Fail)]
pub enum HsError {
    #[fail(display = "A parameter passed to this function was invalid")]
    Invalid,
    #[fail(display = "A memory allocation failed.")]
    NoMemory,
    #[fail(display = "The engine was terminated by callback.")]
    ScanTerminated,
    #[fail(display = "The pattern compiler failed with more detail, #{} {}.", _0, _1)]
    CompilerError(usize, String),
    #[fail(display = "The given database was built for a different version of Hyperscan.")]
    DbVersionError,
    #[fail(display = "The given database was built for a different platform (i.e., CPU type).")]
    DbPlatformError,
    #[fail(display = "The given database was built for a different mode of operation.")]
    DbModeError,
    #[fail(display = "A parameter passed to this function was not correctly aligned.")]
    BadAlign,
    #[fail(display = "The memory allocator did not correctly return memory suitably aligned")]
    BadAlloc,
    #[fail(display = "The scratch region was already in use.")]
    ScratchInUse,
    #[fail(display = "Unsupported CPU architecture.")]
    UnsupportedArch,
    #[fail(display = "Provided buffer was too small.")]
    InsufficientSpace,
    #[fail(display = "Unknown error code: {}", _0)]
    Failed(i32),
}

impl From<i32> for HsError {
    fn from(result: i32) -> Self {
        match result {
            HS_SUCCESS => {
                unreachable!();
            }
            HS_INVALID => HsError::Invalid,
            HS_NOMEM => HsError::NoMemory,
            HS_SCAN_TERMINATED => HsError::ScanTerminated,
            HS_DB_VERSION_ERROR => HsError::DbVersionError,
            HS_DB_PLATFORM_ERROR => HsError::DbPlatformError,
            HS_DB_MODE_ERROR => HsError::DbModeError,
            HS_BAD_ALIGN => HsError::BadAlign,
            HS_BAD_ALLOC => HsError::BadAlloc,
            HS_SCRATCH_IN_USE => HsError::ScratchInUse,
            HS_ARCH_ERROR => HsError::UnsupportedArch,
            HS_INSUFFICIENT_SPACE => HsError::InsufficientSpace,
            err => HsError::Failed(err),
        }
    }
}

macro_rules! check_hs_error {
    ($result:expr) => {
        if $result != $crate::HS_SUCCESS {
            return Err($crate::errors::HsError::from($result).into());
        }
    };
}

macro_rules! check_scan_error {
    ($result:expr) => {
        match $result {
            $crate::HS_SUCCESS | $crate::HS_SCAN_TERMINATED => {}
            _ => return Err($crate::errors::HsError::from($result).into()),
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

pub type RawCompileErrorPtr = *mut hs_compile_error_t;

/// Providing details of the compile error condition.
#[derive(Debug)]
pub struct CompileError(RawCompileErrorPtr);

unsafe impl Send for CompileError {}
unsafe impl Sync for CompileError {}

impl CompileError {
    pub fn expression(&self) -> usize {
        unsafe { (*self.0).expression as usize }
    }

    pub fn message(&self) -> String {
        unsafe { String::from(CStr::from_ptr((*self.0).message).to_str().unwrap()) }
    }
}

impl Drop for CompileError {
    fn drop(&mut self) {
        unsafe {
            assert_hs_error!(hs_free_compile_error(self.0));
        }
    }
}

impl From<*mut hs_compile_error_t> for CompileError {
    fn from(err: *mut hs_compile_error_t) -> Self {
        CompileError(err)
    }
}

macro_rules! check_compile_error {
    ($result:expr, $err:ident) => {
        match $result {
            $crate::HS_SUCCESS => {}
            $crate::HS_COMPILER_ERROR => {
                let err = $crate::errors::CompileError::from($err);

                trace!("compile expression #{} failed, {}", err.expression(), err.message());

                return Err($crate::errors::HsError::CompilerError(err.expression(), err.message()).into());
            }
            _ => return Err($crate::errors::HsError::from($result).into()),
        }
    };
}
