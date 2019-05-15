use core::mem;
use core::ptr::null_mut;
use std::ffi::CString;

use failure::Error;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};
use libc::c_uint;

use crate::common::{Database, Mode};
use crate::compile::{Pattern, Patterns};
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
        unsafe { ffi::hs_valid_platform().ok().map(|_| ()) }
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
    pub fn compile(expression: &str, flags: u32, platform: Option<&PlatformInfoRef>) -> Result<Database<T>, Error> {
        let expr = CString::new(expression)?;
        let mut db = null_mut();
        let mut err = null_mut();

        unsafe {
            check_compile_error!(
                ffi::hs_compile(
                    expr.as_bytes_with_nul().as_ptr() as *const i8,
                    flags,
                    T::ID,
                    platform.map_or_else(null_mut, |p| p.as_ptr()),
                    &mut db,
                    &mut err
                ),
                err
            );

            Ok(Database::from_ptr(db))
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
        Database::compile(&self.expression, self.flags.0, platform)
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
            flags.push(pattern.flags.0 as c_uint);
            ids.push(pattern.id as c_uint);
        }

        for expr in &expressions {
            ptrs.push(expr.as_bytes_with_nul().as_ptr() as *const i8);
        }

        let mut db = null_mut();
        let mut err = null_mut();

        unsafe {
            check_compile_error!(
                ffi::hs_compile_multi(
                    ptrs.as_ptr(),
                    flags.as_ptr(),
                    ids.as_ptr(),
                    self.len() as u32,
                    T::ID,
                    platform.map_or_else(null_mut, |p| p.as_ptr()),
                    &mut db,
                    &mut err
                ),
                err
            );

            Ok(Database::from_ptr(db))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::common::tests::*;
    use crate::common::*;
    use crate::compile::*;
    use crate::constants::*;

    const DATABASE_SIZE: usize = 2664;

    #[test]
    fn test_compile_flags() {
        let _ = pretty_env_logger::try_init();

        let mut flags = CompileFlags(HS_FLAG_CASELESS | HS_FLAG_DOTALL);

        assert_eq!(flags, CompileFlags(HS_FLAG_CASELESS | HS_FLAG_DOTALL));
        assert!(flags.is_set(HS_FLAG_CASELESS));
        assert!(!flags.is_set(HS_FLAG_MULTILINE));
        assert!(flags.is_set(HS_FLAG_DOTALL));
        assert_eq!(format!("{}", flags), "is");

        assert_eq!(
            *flags.set(HS_FLAG_MULTILINE),
            CompileFlags(HS_FLAG_CASELESS | HS_FLAG_MULTILINE | HS_FLAG_DOTALL)
        );

        assert_eq!(CompileFlags::parse("ism").unwrap(), flags);
        assert!(CompileFlags::parse("test").is_err());
    }

    #[test]
    fn test_database_compile() {
        let _ = pretty_env_logger::try_init();
        let info = PlatformInfo::host().unwrap();

        let db = BlockDatabase::compile("test", 0, Some(&info)).unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_pattern() {
        let _ = pretty_env_logger::try_init();

        let p = Pattern::parse("test").unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags(0));
        assert_eq!(p.id, 0);

        let p = Pattern::parse("/test/").unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags(0));
        assert_eq!(p.id, 0);

        let p = Pattern::parse("/test/i").unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags(HS_FLAG_CASELESS));
        assert_eq!(p.id, 0);

        let p = Pattern::parse("3:/test/i").unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags(HS_FLAG_CASELESS));
        assert_eq!(p.id, 3);

        let p = Pattern::parse("test/i").unwrap();

        assert_eq!(p.expression, "test/i");
        assert_eq!(p.flags, CompileFlags(0));
        assert_eq!(p.id, 0);

        let p = Pattern::parse("/t/e/s/t/i").unwrap();

        assert_eq!(p.expression, "t/e/s/t");
        assert_eq!(p.flags, CompileFlags(HS_FLAG_CASELESS));
        assert_eq!(p.id, 0);
    }

    #[test]
    fn test_pattern_build() {
        let _ = pretty_env_logger::try_init();

        let p = &pattern! {"test"};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags(0));
        assert_eq!(p.id, 0);

        let info = p.info().unwrap();

        assert_eq!(info.min_width, 4);
        assert_eq!(info.max_width, 4);
        assert!(!info.unordered_matches);
        assert!(!info.matches_at_eod);
        assert!(!info.matches_only_at_eod);

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_pattern_build_with_flags() {
        let _ = pretty_env_logger::try_init();

        let p = &pattern! {"test", flags => HS_FLAG_CASELESS};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags(HS_FLAG_CASELESS));
        assert_eq!(p.id, 0);

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_patterns_build() {
        let _ = pretty_env_logger::try_init();

        let db: BlockDatabase = patterns!(["test", "foo", "bar"]).build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }

    #[test]
    fn test_patterns_build_with_flags() {
        let _ = pretty_env_logger::try_init();

        let db: BlockDatabase = patterns!(["test", "foo", "bar"], flags => HS_FLAG_CASELESS|HS_FLAG_DOTALL)
            .build()
            .unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }
}
