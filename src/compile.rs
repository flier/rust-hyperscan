use std::ptr;
use std::fmt;
use std::os::raw::c_uint;
use std::str::FromStr;
use std::ffi::CString;
use std::iter::FromIterator;
use std::result::Result as StdResult;

use regex_syntax;

use raw::*;
use constants::*;
use api::*;
use common::{DatabaseType, RawDatabase};
use errors::{Error, RawCompileErrorPtr, Result};

const HS_MODE_SOM_HORIZON_DEFAULT: CompileMode = HS_MODE_SOM_HORIZON_SMALL;

impl Default for CompileFlags {
    fn default() -> Self {
        CompileFlags::empty()
    }
}

impl fmt::Display for CompileFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.contains(HS_FLAG_CASELESS) {
            write!(f, "i")?;
        }
        if self.contains(HS_FLAG_MULTILINE) {
            write!(f, "m")?;
        }
        if self.contains(HS_FLAG_DOTALL) {
            write!(f, "s")?;
        }
        if self.contains(HS_FLAG_SINGLEMATCH) {
            write!(f, "H")?;
        }
        if self.contains(HS_FLAG_ALLOWEMPTY) {
            write!(f, "V")?;
        }
        if self.contains(HS_FLAG_UTF8) {
            write!(f, "8")?;
        }
        if self.contains(HS_FLAG_UCP) {
            write!(f, "W")?;
        }
        Ok(())
    }
}

impl FromStr for CompileFlags {
    type Err = Error;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        let mut flags = CompileFlags::empty();

        for c in s.chars() {
            match c {
                'i' => flags |= HS_FLAG_CASELESS,
                'm' => flags |= HS_FLAG_MULTILINE,
                's' => flags |= HS_FLAG_DOTALL,
                'H' => flags |= HS_FLAG_SINGLEMATCH,
                'V' => flags |= HS_FLAG_ALLOWEMPTY,
                '8' => flags |= HS_FLAG_UTF8,
                'W' => flags |= HS_FLAG_UCP,
                _ => bail!("invalid compile flag: {}", c),
            }
        }

        Ok(flags)
    }
}

/// A structure containing additional parameters related to an expression
///
/// These parameters allow the set of matches produced by a pattern to be constrained at compile time,
/// rather than relying on the application to process unwanted matches at runtime.
#[derive(Debug, Clone)]
pub struct ExpressionExt {
    /// The minimum end offset in the data stream at which this expression should match successfully.
    pub min_offset: Option<u64>,

    /// The maximum end offset in the data stream at which this expression should match successfully.
    pub max_offset: Option<u64>,

    /// The minimum match length (from start to end) required to successfully match this expression.
    pub min_length: Option<u64>,

    /// Allow patterns to approximately match within this edit distance.
    pub edit_distance: Option<u32>,
}

impl ExpressionExt {
    fn to_raw(&self) -> hs_expr_ext_t {
        let flags = self.min_offset.map_or(ExpressionExtFlags::empty(), |_| {
            HS_EXT_FLAG_MIN_OFFSET
        }) |
            self.max_offset.map_or(ExpressionExtFlags::empty(), |_| {
                HS_EXT_FLAG_MAX_OFFSET
            }) |
            self.min_length.map_or(ExpressionExtFlags::empty(), |_| {
                HS_EXT_FLAG_MIN_LENGTH
            }) |
            self.edit_distance.map_or(
                ExpressionExtFlags::empty(),
                |_| HS_EXT_FLAG_EDIT_DISTANCE,
            );

        hs_expr_ext_t {
            flags: flags.bits(),
            min_offset: self.min_offset.unwrap_or_default(),
            max_offset: self.max_offset.unwrap_or_default(),
            min_length: self.min_length.unwrap_or_default(),
            edit_distance: self.edit_distance.unwrap_or_default(),
        }
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
    pub id: Option<usize>,
    /// Ext set of matches produced by a pattern to be constrained at compile time.
    pub ext: Option<ExpressionExt>,
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(id) = self.id {
            write!(
                f,
                "{}:/{}/{}",
                id,
                regex_syntax::escape(self.expression.as_str()),
                self.flags
            )
        } else {
            write!(
                f,
                "/{}/{}",
                regex_syntax::escape(self.expression.as_str()),
                self.flags
            )
        }
    }
}

impl FromStr for Pattern {
    type Err = Error;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        let (id, expr) = s.find(':')
            .and_then(|off| {
                let (id, expr) = s.split_at(off);

                id.parse().ok().map(|n| (Some(n), &expr[1..]))
            })
            .unwrap_or((None, s));

        let pattern = match (expr.starts_with('/'), expr.rfind('/')) {
            (true, Some(end)) if end > 0 => Pattern {
                expression: expr[1..end].into(),
                flags: expr[end + 1..].parse()?,
                id: id,
                ext: None,
            },

            _ => Pattern {
                expression: String::from(expr),
                flags: CompileFlags::empty(),
                id: id,
                ext: None,
            },
        };

        debug!("pattern `{}` parsed to `{}`", s, pattern);

        Ok(pattern)
    }
}

impl Expression for Pattern {
    fn info(&self) -> Result<ExpressionInfo> {
        let expr = CString::new(self.expression.as_str())?;
        let mut info: RawExpressionInfoPtr = ptr::null_mut();
        let mut err: RawCompileErrorPtr = ptr::null_mut();

        unsafe {
            if let Some(ref ext) = self.ext {
                check_compile_error!(
                    hs_expression_ext_info(
                        expr.as_bytes_with_nul().as_ptr() as *const i8,
                        self.flags.bits(),
                        &ext.to_raw(),
                        &mut info,
                        &mut err,
                    ),
                    err
                );
            } else {
                check_compile_error!(
                    hs_expression_info(
                        expr.as_bytes_with_nul().as_ptr() as *const i8,
                        self.flags.bits(),
                        &mut info,
                        &mut err,
                    ),
                    err
                );
            }

            let info = info.into();

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
    ( $expr:expr ) => {{
        $crate::Pattern {
            expression: $expr.into(),
            flags: CompileFlags::default(),
            id: None,
            ext: None,
        }
    }};
    ( $expr:expr, flags => $flags:expr ) => {{
        $crate::Pattern {
            expression: $expr.into(),
            flags: $flags.into(),
            id: None,
            ext: None,
        }
    }};
    ( $expr:expr, flags => $flags:expr, id => $id:expr ) => {{
        $crate::Pattern {
            expression: $expr.into(),
            flags: $flags.into(),
            id: Some($id),
            ext: None,
        }
    }};
    ( $expr:expr, flags => $flags:expr, id => $id:expr, ext => $ext:expr ) => {{
        $crate::Pattern {
            expression: $expr.into(),
            flags: $flags.into(),
            id: Some($id),
            ext: $ext,
        }
    }}
}

/// Define multi `Pattern` with flags and ID
#[macro_export]
macro_rules! patterns {
    [ $( $expr:expr ), * ] => {{
        patterns!([ $( $expr ), * ], flags => CompileFlags::default())
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

impl<T: DatabaseType> RawDatabase<T> {
    /// The basic regular expression compiler.
    ///
    /// This is the function call with which an expression is compiled into a Hyperscan database
    // which can be passed to the runtime functions.
    pub fn compile<S: AsRef<str>>(
        expression: S,
        flags: CompileFlags,
        platform: Option<&PlatformInfo>,
    ) -> Result<RawDatabase<T>> {
        let mode = if T::MODE == HS_MODE_STREAM && flags.contains(HS_FLAG_SOM_LEFTMOST) {
            T::MODE | HS_MODE_SOM_HORIZON_DEFAULT
        } else {
            T::MODE
        };
        let expression = expression.as_ref();
        let cexpr = CString::new(expression)?;
        let mut db: RawDatabasePtr = ptr::null_mut();
        let mut err: RawCompileErrorPtr = ptr::null_mut();

        unsafe {
            check_compile_error!(
                hs_compile(
                    cexpr.as_ptr() as *const i8,
                    flags.bits(),
                    mode.bits(),
                    platform.map(|p| p.as_ptr()).unwrap_or_else(ptr::null),
                    &mut db,
                    &mut err,
                ),
                err
            );
        }

        debug!(
            "pattern `/{}/{}` compiled to {} database {:p}",
            expression,
            flags,
            T::NAME,
            db
        );

        Ok(RawDatabase::from_raw(db))
    }
}

impl<T: DatabaseType> DatabaseBuilder<RawDatabase<T>> for Pattern {
    ///
    /// The basic regular expression compiler.
    ///
    /// / This is the function call with which an expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions
    ///
    fn build_for_platform(&self, platform: Option<&PlatformInfo>) -> Result<RawDatabase<T>> {
        RawDatabase::compile(&self.expression, self.flags, platform)
    }
}

impl<T: DatabaseType> DatabaseBuilder<RawDatabase<T>> for Vec<Pattern> {
    ///
    /// The multiple regular expression compiler.
    ///
    /// This is the function call with which a set of expressions is compiled into a database
    /// which can be passed to the runtime functions.
    /// Each expression can be labelled with a unique integer
    // which is passed into the match callback to identify the pattern that has matched.
    ///
    fn build_for_platform(&self, platform: Option<&PlatformInfo>) -> Result<RawDatabase<T>> {
        self.as_slice().build_for_platform(platform)
    }
}

impl<'a, T: DatabaseType> DatabaseBuilder<RawDatabase<T>> for &'a [Pattern] {
    ///
    /// The multiple regular expression compiler.
    ///
    /// This is the function call with which a set of expressions is compiled into a database
    /// which can be passed to the runtime functions.
    /// Each expression can be labelled with a unique integer
    // which is passed into the match callback to identify the pattern that has matched.
    ///
    fn build_for_platform(&self, platform: Option<&PlatformInfo>) -> Result<RawDatabase<T>> {
        let mode = if T::MODE == HS_MODE_STREAM &&
            self.iter().any(|pattern| {
                pattern.flags.contains(HS_FLAG_SOM_LEFTMOST)
            })
        {
            T::MODE | HS_MODE_SOM_HORIZON_DEFAULT
        } else {
            T::MODE
        };
        let mut exprs = Vec::with_capacity(self.len());
        let mut flags = Vec::with_capacity(self.len());
        let mut ids = Vec::with_capacity(self.len());
        let mut exts = Vec::with_capacity(self.len());

        for pattern in self.iter() {
            exprs.push(CString::new(pattern.expression.as_str())?);
            flags.push(pattern.flags.bits());
            ids.push(pattern.id.unwrap_or_default() as c_uint);
            exts.push(pattern.ext.as_ref().map(|ext| ext.to_raw()));
        }

        let expr_ptrs = exprs
            .iter()
            .map(|expr| expr.as_ptr() as *const i8)
            .collect::<Vec<*const i8>>();

        let mut db: RawDatabasePtr = ptr::null_mut();
        let mut err: RawCompileErrorPtr = ptr::null_mut();

        unsafe {
            if exts.iter().any(|ptr| ptr.is_some()) {
                let ext_ptrs = exts.iter()
                    .map(|ext| ext.map_or_else(ptr::null, |ext| &ext))
                    .collect::<Vec<*const hs_expr_ext_t>>();

                check_compile_error!(
                    hs_compile_ext_multi(
                        expr_ptrs.as_ptr(),
                        flags.as_ptr(),
                        ids.as_ptr(),
                        ext_ptrs.as_ptr(),
                        self.len() as u32,
                        mode.bits(),
                        platform.map(|p| p.as_ptr()).unwrap_or_else(ptr::null),
                        &mut db,
                        &mut err,
                    ),
                    err
                );
            } else {
                check_compile_error!(
                    hs_compile_multi(
                        expr_ptrs.as_ptr(),
                        flags.as_ptr(),
                        ids.as_ptr(),
                        self.len() as u32,
                        mode.bits(),
                        platform.map(|p| p.as_ptr()).unwrap_or_else(ptr::null),
                        &mut db,
                        &mut err,
                    ),
                    err
                );
            }
        }

        debug!(
            "patterns [{}] compiled to {} database {:p}",
            Vec::from_iter(self.iter().map(|p| format!("`{}`", p))).join(", "),
            T::NAME,
            db
        );

        Ok(RawDatabase::from_raw(db))
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;

    use super::super::*;
    use errors::{Error, ErrorKind};
    use raw::AsPtr;
    use common::tests::*;

    const DATABASE_SIZE: usize = 2664;

    #[test]
    fn test_compile_flags() {
        let flags = HS_FLAG_CASELESS | HS_FLAG_DOTALL;

        assert!(flags.contains(HS_FLAG_CASELESS));
        assert!(!flags.contains(HS_FLAG_MULTILINE));
        assert!(flags.contains(HS_FLAG_DOTALL));
        assert_eq!(flags.to_string(), "is");

        assert_eq!(flags, HS_FLAG_CASELESS | HS_FLAG_DOTALL);

        assert_eq!(
            "ism".parse::<CompileFlags>().unwrap(),
            flags | HS_FLAG_MULTILINE
        );
        assert_matches!(
            "test".parse::<CompileFlags>().err().unwrap(),
            Error(ErrorKind::Msg(_), _)
        );
    }

    #[test]
    fn test_database_compile() {
        let db = BlockDatabase::compile(
            "test",
            CompileFlags::default(),
            PlatformInfo::populate().ok().as_ref(),
        ).unwrap();

        assert!(db.as_ptr() != ptr::null_mut());

        validate_database(&db);
    }

    #[test]
    fn test_pattern() {
        let p: Pattern = "test".parse().unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags::empty());
        assert_eq!(p.id, None);

        let p: Pattern = "/test/".parse().unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags::empty());
        assert_eq!(p.id, None);

        let p: Pattern = "/test/i".parse().unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, HS_FLAG_CASELESS);
        assert_eq!(p.id, None);

        let p: Pattern = "3:/test/i".parse().unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, HS_FLAG_CASELESS);
        assert_eq!(p.id, Some(3));

        let p: Pattern = "test/i".parse().unwrap();

        assert_eq!(p.expression, "test/i");
        assert_eq!(p.flags, CompileFlags::empty());
        assert_eq!(p.id, None);

        let p: Pattern = "/t/e/s/t/i".parse().unwrap();

        assert_eq!(p.expression, "t/e/s/t");
        assert_eq!(p.flags, HS_FLAG_CASELESS);
        assert_eq!(p.id, None);
    }

    #[test]
    fn test_pattern_build() {
        let p = &pattern!{"test"};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, CompileFlags::empty());
        assert_eq!(p.id, None);

        let info = p.info().unwrap();

        assert_eq!(info.min_width(), 4);
        assert_eq!(info.max_width(), 4);
        assert!(!info.unordered_matches());
        assert!(!info.matches_at_eod());
        assert!(!info.matches_only_at_eod());

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_pattern_build_with_flags() {
        let p = &pattern!{"test", flags => HS_FLAG_CASELESS};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, HS_FLAG_CASELESS);
        assert_eq!(p.id, None);

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_patterns_build() {
        let db: BlockDatabase = patterns!["test", "foo", "bar"].build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }

    #[test]
    fn test_patterns_build_with_flags() {
        let db: BlockDatabase = patterns!(["test", "foo", "bar"], flags => HS_FLAG_CASELESS | HS_FLAG_DOTALL)
            .build()
            .unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }
}
