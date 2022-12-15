use thiserror::Error;

use crate::ffi;

/// Hyperscan Error Codes
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// A parameter passed to this function was invalid.
    #[error("A parameter passed to this function was invalid.")]
    Invalid,

    /// A memory allocation failed.
    #[error("A memory allocation failed.")]
    NoMem,

    /// The engine was terminated by callback.
    #[error("The engine was terminated by callback.")]
    ScanTerminated,

    /// The pattern compiler failed with more detail.
    #[cfg(feature = "compile")]
    #[error("The pattern compiler failed with more detail, {0}.")]
    CompileError(crate::compile::Error),

    /// The given database was built for a different version of Hyperscan.
    #[error("The given database was built for a different version of Hyperscan.")]
    DbVersionError,

    /// The given database was built for a different platform (i.e., CPU type).
    #[error("The given database was built for a different platform (i.e., CPU type).")]
    DbPlatformError,

    /// The given database was built for a different mode of operation.
    #[error("The given database was built for a different mode of operation.")]
    DbModeError,

    /// A parameter passed to this function was not correctly aligned.
    #[error("A parameter passed to this function was not correctly aligned.")]
    BadAlign,

    /// The memory allocator did not correctly return memory suitably aligned.
    #[error("The memory allocator did not correctly return memory suitably aligned.")]
    BadAlloc,

    /// The scratch region was already in use.
    #[error("The scratch region was already in use.")]
    ScratchInUse,

    /// Unsupported CPU architecture.
    #[error("Unsupported CPU architecture.")]
    ArchError,

    /// Provided buffer was too small.
    #[error("Provided buffer was too small.")]
    InsufficientSpace,

    /// Unexpected internal error.
    #[cfg(feature = "v5")]
    #[error("Unexpected internal error.")]
    UnknownError,

    /// Unknown error code
    #[error("Unknown error code: {0}")]
    Code(ffi::hs_error_t),
}

impl From<ffi::hs_error_t> for Error {
    fn from(err: ffi::hs_error_t) -> Self {
        use Error::*;

        match err {
            ffi::HS_INVALID => Invalid,
            ffi::HS_NOMEM => NoMem,
            ffi::HS_SCAN_TERMINATED => ScanTerminated,
            // ffi::HS_COMPILER_ERROR => HsError::CompileError,
            ffi::HS_DB_VERSION_ERROR => DbVersionError,
            ffi::HS_DB_PLATFORM_ERROR => DbPlatformError,
            ffi::HS_DB_MODE_ERROR => DbModeError,
            ffi::HS_BAD_ALIGN => BadAlign,
            ffi::HS_BAD_ALLOC => BadAlloc,
            ffi::HS_SCRATCH_IN_USE => ScratchInUse,
            ffi::HS_ARCH_ERROR => ArchError,
            ffi::HS_INSUFFICIENT_SPACE => InsufficientSpace,
            #[cfg(feature = "v5")]
            ffi::HS_UNKNOWN_ERROR => UnknownError,
            _ => Code(err),
        }
    }
}
