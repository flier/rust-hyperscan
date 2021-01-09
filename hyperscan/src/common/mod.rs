mod database;
mod error;
mod mode;
mod serialized;

pub use self::database::{BlockDatabase, Database, DatabaseRef, StreamingDatabase, VectoredDatabase};
pub use self::error::Error;
pub use self::mode::{Block, Mode, Streaming, Vectored};
pub use self::serialized::Serialized;

#[cfg(test)]
pub mod tests {
    pub use super::database::tests::*;
}

use std::ffi::CStr;

use crate::ffi;

/// The current Hyperscan version information.
///
/// # Examples
///
/// ```rust
/// assert!(hyperscan::version_str().to_string_lossy().starts_with(&hyperscan::version().to_string()));
/// ```
pub fn version() -> semver::Version {
    semver::Version::parse(version_str().to_string_lossy().split(' ').next().unwrap()).unwrap()
}

/// Utility function for identifying this release version.
///
/// Returns a string containing the version number of this release build  and the date of the build.
pub fn version_str() -> &'static CStr {
    unsafe { CStr::from_ptr(ffi::hs_version()) }
}
