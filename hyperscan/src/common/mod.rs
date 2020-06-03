mod database;
mod mode;
mod serialized;

pub use self::database::{BlockDatabase, Database, DatabaseRef, StreamingDatabase, VectoredDatabase};
pub use self::mode::{Block, Mode, Streaming, Vectored};
pub use self::serialized::Serialized;

#[cfg(test)]
pub mod tests {
    pub use super::database::tests::*;
}

use std::ffi::CStr;

use crate::ffi;

/// The current Hyperscan version information.
pub fn version() -> semver::Version {
    semver::Version::new(ffi::HS_MAJOR as u64, ffi::HS_MINOR as u64, ffi::HS_PATCH as u64)
}

/// Utility function for identifying this release version.
///
/// Returns a string containing the version number of this release build  and the date of the build.
pub fn version_str() -> &'static CStr {
    unsafe { CStr::from_ptr(ffi::hs_version()) }
}
