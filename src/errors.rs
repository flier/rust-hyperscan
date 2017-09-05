use std::fmt;
use std::string::ToString;
use std::ffi::CStr;

use constants::*;
use raw::*;

error_chain! {
    foreign_links {
        ParseError(::std::num::ParseIntError) #[doc="An error which can be returned when parsing an integer."];

        NulError(::std::ffi::NulError);

        Utf8Error(::std::str::Utf8Error);
    }

    errors {
        HsError(err: HsError) {
            description("hyperscan error")
            display("hyperscan error, {}", err)
        }

        CompileError(err: CompileError) {
            description("compile error")
            display("compile expression #{} failed, {}", err.expression(), err.message())
        }
    }
}

#[derive(Debug)]
pub enum HsError {
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
    CompilerError,
    /// The given database was built for a different version of Hyperscan.
    DbVersionError,
    /// The given database was built for a different platform (i.e., CPU type).
    DbPlatformError,
    /// The given database was built for a different mode of operation.
    /// This error is returned when streaming calls are used
    /// with a block or vectored database and vice versa.
    DbModeError,
    /// A parameter passed to this function was not correctly aligned.
    BadAlign,
    /// The memory allocator (either malloc() or the allocator set with hs_set_allocator())
    /// did not correctly return memory suitably aligned
    /// for the largest representable data type on this platform.
    BadAlloc,
    /// The scratch region was already in use.
    ScratchInUse,
    /// Unsupported CPU architecture.
    UnsupportedArch,
    /// Unknown error code
    Failed(i32),
}

impl From<i32> for HsError {
    fn from(result: i32) -> Self {
        match result {
            HS_SUCCESS => HsError::Success,
            HS_INVALID => HsError::Invalid,
            HS_NOMEM => HsError::NoMem,
            HS_SCAN_TERMINATED => HsError::ScanTerminated,
            HS_COMPILER_ERROR => HsError::CompilerError,
            HS_DB_VERSION_ERROR => HsError::DbVersionError,
            HS_DB_PLATFORM_ERROR => HsError::DbPlatformError,
            HS_DB_MODE_ERROR => HsError::DbModeError,
            HS_BAD_ALIGN => HsError::BadAlign,
            HS_BAD_ALLOC => HsError::BadAlloc,
            HS_SCRATCH_IN_USE => HsError::ScratchInUse,
            HS_ARCH_ERROR => HsError::UnsupportedArch,
            err => HsError::Failed(err),
        }
    }
}

impl fmt::Display for HsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            HsError::Success => write!(f, "the engine completed normally."),
            HsError::Invalid => write!(f, "a parameter passed to this function was invalid."),
            HsError::NoMem => write!(f, "a memory allocation failed."),
            HsError::ScanTerminated => write!(f, "the engine was terminated by callback."),
            HsError::CompilerError => write!(f, "the pattern compiler failed"),
            HsError::DbVersionError => write!(
                f,
                "the given database was built for a different version of Hyperscan."
            ),
            HsError::DbPlatformError => write!(f, "the given database was built for a different platform."),
            HsError::DbModeError => write!(
                f,
                "the given database was built for a different mode of operation."
            ),
            HsError::BadAlign => write!(
                f,
                "a parameter passed to this function was not correctly aligned."
            ),
            HsError::BadAlloc => write!(
                f,
                "the memory allocator did not correctly return memory suitably aligned."
            ),
            HsError::ScratchInUse => write!(f, "the scratch region was already in use."),
            HsError::UnsupportedArch => write!(f, "unsupported CPU architecture."),
            HsError::Failed(err) => write!(f, "internal operation failed, error code {}.", err),
        }
    }
}

macro_rules! check_hs_error {
    ($result:expr) => (if $result != $crate::HS_SUCCESS {
        bail!($crate::errors::ErrorKind::HsError($result.into()))
    })
}

macro_rules! assert_hs_error {
    ($expr:expr) => (if $expr != $crate::HS_SUCCESS {
        panic!("panic, err={}", $expr);
    })
}

pub type RawCompileErrorPtr = *mut hs_compile_error_t;

/// Providing details of the compile error condition.
#[derive(Debug)]
pub struct CompileError(RawCompileErrorPtr);

unsafe impl Send for CompileError {}

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
        if $crate::HS_SUCCESS != $result {
            match $result {
                $crate::HS_COMPILER_ERROR => {
                    let err: $crate::errors::CompileError = $err.into();

                    trace!("compile expression #{} failed, {}", err.expression(), err.message());

                    bail!($crate::errors::ErrorKind::CompileError(err))
                },
                _ => {
                    bail!($crate::errors::ErrorKind::HsError($result.into()))
                },
            }
        }
    }
}
