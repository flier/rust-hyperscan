use std::ffi::CString;
use std::fmt;
use std::iter::FromIterator;
use std::os::raw::c_uint;
use std::ptr;
use std::str::FromStr;

use regex_syntax;

use api::*;
use common::RawDatabase;
use constants::*;
use cptr::CPtr;
use errors::{Error, RawCompileErrorPtr};
use raw::*;

/// Flags which modify the behaviour of the expression.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct CompileFlags(pub u32);

impl From<u32> for CompileFlags {
    fn from(flags: u32) -> Self {
        CompileFlags(flags)
    }
}

impl Into<u32> for CompileFlags {
    fn into(self) -> u32 {
        self.0
    }
}

impl fmt::Display for CompileFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_set(HS_FLAG_CASELESS) {
            try!(write!(f, "i"))
        }
        if self.is_set(HS_FLAG_MULTILINE) {
            try!(write!(f, "m"))
        }
        if self.is_set(HS_FLAG_DOTALL) {
            try!(write!(f, "s"))
        }
        if self.is_set(HS_FLAG_SINGLEMATCH) {
            try!(write!(f, "H"))
        }
        if self.is_set(HS_FLAG_ALLOWEMPTY) {
            try!(write!(f, "V"))
        }
        if self.is_set(HS_FLAG_UTF8) {
            try!(write!(f, "8"))
        }
        if self.is_set(HS_FLAG_UCP) {
            try!(write!(f, "W"))
        }
        if self.is_set(HS_FLAG_COMBINATION) {
            try!(write!(f, "C"))
        }
        if self.is_set(HS_FLAG_QUIET) {
            try!(write!(f, "Q"))
        }
        Ok(())
    }
}

impl CompileFlags {
    #[inline]
    pub fn is_set(self, flag: u32) -> bool {
        self.0 & flag == flag
    }

    #[inline]
    pub fn set(&mut self, flag: u32) -> &mut Self {
        self.0 |= flag;

        self
    }

    pub fn parse(s: &str) -> Result<CompileFlags, Error> {
        let mut flags: u32 = 0;

        for c in s.chars() {
            match c {
                'i' => flags |= HS_FLAG_CASELESS,
                'm' => flags |= HS_FLAG_MULTILINE,
                's' => flags |= HS_FLAG_DOTALL,
                'H' => flags |= HS_FLAG_SINGLEMATCH,
                'V' => flags |= HS_FLAG_ALLOWEMPTY,
                '8' => flags |= HS_FLAG_UTF8,
                'W' => flags |= HS_FLAG_UCP,
                'C' => flags |= HS_FLAG_COMBINATION,
                'Q' => flags |= HS_FLAG_QUIET,
                _ => return Err(Error::CompilerError(format!("invalid compile flag: {}", c))),
            }
        }

        Ok(CompileFlags(flags))
    }
}

impl FromStr for CompileFlags {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CompileFlags::parse(s)
    }
}

/// Pattern that has matched.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// The NULL-terminated expression to parse.
    pub expression: String,
    /// Flags which modify the behaviour of the expression.
    pub flags: CompileFlags,
    /// ID number to be associated with the corresponding pattern in the expressions array.
    pub id: usize,
}

impl Pattern {
    pub fn parse(s: &str) -> Result<Pattern, Error> {
        let (id, expr) = match s.find(':') {
            Some(off) => (s[0..off].parse()?, &s[off + 1..]),
            None => (0, s),
        };

        let pattern = match (expr.starts_with('/'), expr.rfind('/')) {
            (true, Some(end)) if end > 0 => Pattern {
                expression: expr[1..end].to_owned(),
                flags: CompileFlags::parse(&expr[end + 1..])?,
                id,
            },

            _ => Pattern {
                expression: String::from(expr),
                flags: CompileFlags::default(),
                id,
            },
        };

        debug!("pattern `{}` parsed to `{}`", s, pattern);

        Ok(pattern)
    }
}

impl fmt::Display for Pattern {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}:/{}/{}",
            self.id,
            regex_syntax::escape(self.expression.as_str()),
            self.flags
        )
    }
}

impl FromStr for Pattern {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Pattern::parse(s)
    }
}

impl Expression for Pattern {
    fn info(&self) -> Result<ExpressionInfo, Error> {
        let expr = try!(CString::new(self.expression.as_str()));
        let mut info: CPtr<hs_expr_info_t> = CPtr::null();
        let mut err: RawCompileErrorPtr = ptr::null_mut();

        unsafe {
            check_compile_error!(
                hs_expression_info(
                    expr.as_bytes_with_nul().as_ptr() as *const i8,
                    self.flags.0,
                    &mut *info,
                    &mut err
                ),
                err
            );

            let info = ExpressionInfo {
                min_width: info.as_ref().min_width as usize,
                max_width: info.as_ref().max_width as usize,
                unordered_matches: info.as_ref().unordered_matches != 0,
                matches_at_eod: info.as_ref().matches_at_eod != 0,
                matches_only_at_eod: info.as_ref().matches_only_at_eod != 0,
            };

            debug!("expression `{}` info: {:?}", self, info);

            Ok(info)
        }
    }
}

/// Vec of `Pattern`
pub type Patterns = Vec<Pattern>;

/// Define `Pattern` with flags
#[macro_export]
macro_rules! pattern {
    ($expr:expr) => {{
        pattern!($expr, flags => 0, id => 0)
    }};
    ($expr:expr,flags => $flags:expr) => {{
        pattern!($expr, flags => $flags, id => 0)
    }};
    ($expr:expr,flags => $flags:expr,id => $id:expr) => {{
        $crate::Pattern {
            expression: ::std::convert::From::from($expr),
            flags: ::std::convert::From::from($flags),
            id: $id,
        }
    }};
}

/// Define multi `Pattern` with flags and ID
#[macro_export]
macro_rules! patterns {
    ( [ $( $expr:expr ), * ] ) => {{
        patterns!([ $( $expr ), * ], flags => 0)
    }};
    ( [ $( $expr:expr ), * ], flags => $flags:expr ) => {{
        let mut v = Vec::new();
        $(
            let id = v.len() + 1;

            v.push(pattern!{$expr, flags => $flags, id => id});
        )*

        v
    }};
}

impl<T: Type> RawDatabase<T> {
    /// The basic regular expression compiler.
    ///
    /// This is the function call with which an expression is compiled into a Hyperscan database
    // which can be passed to the runtime functions.
    pub fn compile(expression: &str, flags: u32, platform: &PlatformInfo) -> Result<RawDatabase<T>, Error> {
        let expr = try!(CString::new(expression));
        let mut db: RawDatabasePtr = ptr::null_mut();
        let mut err: RawCompileErrorPtr = ptr::null_mut();

        unsafe {
            check_compile_error!(
                hs_compile(
                    expr.as_bytes_with_nul().as_ptr() as *const i8,
                    flags,
                    T::mode(),
                    platform.as_ptr(),
                    &mut db,
                    &mut err
                ),
                err
            );
        }

        debug!(
            "pattern `/{}/{}` compiled to {} database {:p}",
            expression,
            CompileFlags(flags),
            T::name(),
            db
        );

        Ok(RawDatabase::from_raw(db))
    }
}

impl<T: Type> DatabaseBuilder<RawDatabase<T>> for Pattern {
    ///
    /// The basic regular expression compiler.
    ///
    /// / This is the function call with which an expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions
    ///
    fn build_for_platform(&self, platform: &PlatformInfo) -> Result<RawDatabase<T>, Error> {
        RawDatabase::compile(&self.expression, self.flags.0, platform)
    }
}

impl<T: Type> DatabaseBuilder<RawDatabase<T>> for Patterns {
    ///
    /// The multiple regular expression compiler.
    ///
    /// This is the function call with which a set of expressions is compiled into a database
    /// which can be passed to the runtime functions.
    /// Each expression can be labelled with a unique integer
    // which is passed into the match callback to identify the pattern that has matched.
    ///
    fn build_for_platform(&self, platform: &PlatformInfo) -> Result<RawDatabase<T>, Error> {
        let mut expressions = Vec::with_capacity(self.len());
        let mut ptrs = Vec::with_capacity(self.len());
        let mut flags = Vec::with_capacity(self.len());
        let mut ids = Vec::with_capacity(self.len());

        for pattern in self {
            let expr = try!(CString::new(pattern.expression.as_str()));

            expressions.push(expr);
            flags.push(pattern.flags.0 as c_uint);
            ids.push(pattern.id as c_uint);
        }

        for expr in &expressions {
            ptrs.push(expr.as_bytes_with_nul().as_ptr() as *const i8);
        }

        let mut db: RawDatabasePtr = ptr::null_mut();
        let mut err: RawCompileErrorPtr = ptr::null_mut();

        unsafe {
            check_compile_error!(
                hs_compile_multi(
                    ptrs.as_ptr(),
                    flags.as_ptr(),
                    ids.as_ptr(),
                    self.len() as u32,
                    T::mode(),
                    platform.as_ptr(),
                    &mut db,
                    &mut err
                ),
                err
            );
        }

        debug!(
            "patterns [{}] compiled to {} database {:p}",
            Vec::from_iter(self.iter().map(|p| format!("`{}`", p))).join(", "),
            T::name(),
            db
        );

        Ok(RawDatabase::from_raw(db))
    }
}

#[cfg(test)]
pub mod tests {
    extern crate env_logger;

    use std::ptr;

    use super::super::common::tests::*;
    use super::super::*;

    const DATABASE_SIZE: usize = 2664;

    #[test]
    fn test_compile_flags() {
        let _ = env_logger::try_init();

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
        let _ = env_logger::try_init();

        let db = BlockDatabase::compile("test", 0, &PlatformInfo::host()).unwrap();

        assert!(*db != ptr::null_mut());

        validate_database(&db);
    }

    #[test]
    fn test_pattern() {
        let _ = env_logger::try_init();

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
        let _ = env_logger::try_init();

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
        let _ = env_logger::try_init();

        let p = &pattern! {"test", flags => HS_FLAG_CASELESS};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags(HS_FLAG_CASELESS));
        assert_eq!(p.id, 0);

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_patterns_build() {
        let _ = env_logger::try_init();

        let db: BlockDatabase = patterns!(["test", "foo", "bar"]).build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }

    #[test]
    fn test_patterns_build_with_flags() {
        let _ = env_logger::try_init();

        let db: BlockDatabase = patterns!(["test", "foo", "bar"], flags => HS_FLAG_CASELESS|HS_FLAG_DOTALL)
            .build()
            .unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }
}
