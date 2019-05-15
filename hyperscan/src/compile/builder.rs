use core::mem;
use core::ptr::null_mut;
use std::ffi::CString;

use failure::Error;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};
use libc::c_uint;

use crate::common::{Database, Mode};
use crate::compile::{AsCompileResult, Flags, Pattern, Patterns};
use crate::errors::AsResult;

foreign_type! {
    /// A type containing information on the target platform
    /// which may optionally be provided to the compile calls
    pub type PlatformInfo {
        type CType = ffi::hs_platform_info_t;

        fn drop = free_platform_info;
    }
}

unsafe fn free_platform_info(p: *mut ffi::hs_platform_info_t) {
    let _ = Box::from_raw(p);
}

impl PlatformInfo {
    pub fn is_valid() -> Result<(), Error> {
        unsafe { ffi::hs_valid_platform().ok() }
    }

    pub fn host() -> Result<PlatformInfo, Error> {
        unsafe {
            let mut platform = mem::zeroed();

            ffi::hs_populate_platform(&mut platform)
                .ok()
                .map(|_| PlatformInfo::from_ptr(Box::into_raw(Box::new(platform))))
        }
    }

    pub fn new(tune: u32, cpu_features: u64) -> PlatformInfo {
        unsafe {
            PlatformInfo::from_ptr(Box::into_raw(Box::new(ffi::hs_platform_info_t {
                tune,
                cpu_features,
                reserved1: 0,
                reserved2: 0,
            })))
        }
    }
}

impl<T: Mode> Database<T> {
    /// The basic regular expression compiler.
    ///
    /// This is the function call with which an expression is compiled into a Hyperscan database
    // which can be passed to the runtime functions.
    pub fn compile<S: AsRef<str>>(
        expression: S,
        flags: Flags,
        platform: Option<&PlatformInfoRef>,
    ) -> Result<Database<T>, Error> {
        let expr = CString::new(expression.as_ref())?;
        let mut db = null_mut();
        let mut err = null_mut();

        unsafe {
            ffi::hs_compile(
                expr.as_bytes_with_nul().as_ptr() as *const i8,
                flags.bits(),
                T::ID,
                platform.map_or_else(null_mut, |p| p.as_ptr()),
                &mut db,
                &mut err,
            )
            .ok_or(err)
            .map(|_| Database::from_ptr(db))
        }
    }
}

/// The regular expression pattern database builder.
pub trait Builder<T> {
    /// This is the function call with which an expression is compiled into
    /// a Hyperscan database which can be passed to the runtime functions
    fn build(&self) -> Result<Database<T>, Error> {
        self.build_for_platform(None)
    }

    fn build_for_platform(&self, platform: Option<&PlatformInfoRef>) -> Result<Database<T>, Error>;
}

impl<T: Mode> Builder<T> for Pattern {
    ///
    /// The basic regular expression compiler.
    ///
    /// / This is the function call with which an expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions
    ///
    fn build_for_platform(&self, platform: Option<&PlatformInfoRef>) -> Result<Database<T>, Error> {
        Database::compile(&self.expression, self.flags, platform)
    }
}

impl<T: Mode> Builder<T> for Patterns {
    ///
    /// The multiple regular expression compiler.
    ///
    /// This is the function call with which a set of expressions is compiled into a database
    /// which can be passed to the runtime functions.
    /// Each expression can be labelled with a unique integer
    // which is passed into the match callback to identify the pattern that has matched.
    ///
    fn build_for_platform(&self, platform: Option<&PlatformInfoRef>) -> Result<Database<T>, Error> {
        let mut expressions = Vec::with_capacity(self.len());
        let mut ptrs = Vec::with_capacity(self.len());
        let mut flags = Vec::with_capacity(self.len());
        let mut ids = Vec::with_capacity(self.len());

        for pattern in self {
            let expr = CString::new(pattern.expression.as_str())?;

            expressions.push(expr);
            flags.push(pattern.flags.bits() as c_uint);
            ids.push(pattern.id as c_uint);
        }

        for expr in &expressions {
            ptrs.push(expr.as_bytes_with_nul().as_ptr() as *const i8);
        }

        let mut db = null_mut();
        let mut err = null_mut();

        unsafe {
            ffi::hs_compile_multi(
                ptrs.as_ptr(),
                flags.as_ptr(),
                ids.as_ptr(),
                self.len() as u32,
                T::ID,
                platform.map_or_else(null_mut, |p| p.as_ptr()),
                &mut db,
                &mut err,
            )
            .ok_or(err)
            .map(|_| Database::from_ptr(db))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::common::tests::*;
    use crate::common::*;
    use crate::compile::Flags;

    #[test]
    fn test_database_compile() {
        let _ = pretty_env_logger::try_init();
        let info = PlatformInfo::host().unwrap();

        let db = BlockDatabase::compile("test", Flags::empty(), Some(&info)).unwrap();

        validate_database(&db);
    }
}
