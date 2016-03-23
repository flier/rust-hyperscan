use std::ptr;
use std::mem;

use raw::*;
use common::{Type, RawDatabase, Error};

#[macro_export]
macro_rules! check_compile_error {
    ($expr:expr, $err:ident) => {
        if $crate::common::HS_SUCCESS != $expr {
            return match $expr {
                $crate::common::HS_COMPILER_ERROR => {
                    let msg = $crate::std::ffi::CString::from_raw((*$err).message).into_string().unwrap();

                    $crate::std::result::Result::Err($crate::common::Error::CompilerError(msg))
                },
                _ =>
                    $crate::std::result::Result::Err($crate::std::convert::From::from($expr)),
            }
        }
    }
}

/**
 * Compile flag: Set case-insensitive matching.
 *
 * This flag sets the expression to be matched case-insensitively by default.
 * The expression may still use PCRE tokens (notably `(?i)` and
 * `(?-i)`) to switch case-insensitive matching on and off.
 */
pub const HS_FLAG_CASELESS: u32 = 1;

/**
 * Compile flag: Matching a `.` will not exclude newlines.
 *
 * This flag sets any instances of the `.` token to match newline characters as
 * well as all other characters. The PCRE specification states that the `.`
 * token does not match newline characters by default, so without this flag the
 * `.` token will not cross line boundaries.
 */
pub const HS_FLAG_DOTALL: u32 = 2;

/**
 * Compile flag: Set multi-line anchoring.
 *
 * This flag instructs the expression to make the `^` and `$` tokens match
 * newline characters as well as the start and end of the stream. If this flag
 * is not specified, the `^` token will only ever match at the start of a
 * stream, and the `$` token will only ever match at the end of a stream within
 * the guidelines of the PCRE specification.
 */
pub const HS_FLAG_MULTILINE: u32 = 4;

/**
 * Compile flag: Set single-match only mode.
 *
 * This flag sets the expression's match ID to match at most once. In streaming
 * mode, this means that the expression will return only a single match over
 * the lifetime of the stream, rather than reporting every match as per
 * standard Hyperscan semantics. In block mode or vectored mode, only the first
 * match for each invocation of @ref hs_scan() or @ref hs_scan_vector() will be
 * returned.
 *
 * If multiple expressions in the database share the same match ID, then they
 * either must all specify @ref HS_FLAG_SINGLEMATCH or none of them specify
 * @ref HS_FLAG_SINGLEMATCH. If a group of expressions sharing a match ID
 * specify the flag, then at most one match with the match ID will be generated
 * per stream.
 *
 * Note: The use of this flag in combination with @ref HS_FLAG_SOM_LEFTMOST
 * is not currently supported.
 */
pub const HS_FLAG_SINGLEMATCH: u32 = 8;

/**
 * Compile flag: Allow expressions that can match against empty buffers.
 *
 * This flag instructs the compiler to allow expressions that can match against
 * empty buffers, such as `.?`, `.*`, `(a|)`. Since Hyperscan can return every
 * possible match for an expression, such expressions generally execute very
 * slowly; the default behaviour is to return an error when an attempt to
 * compile one is made. Using this flag will force the compiler to allow such
 * an expression.
 */
pub const HS_FLAG_ALLOWEMPTY: u32 = 16;

/**
 * Compile flag: Enable UTF-8 mode for this expression.
 *
 * This flag instructs Hyperscan to treat the pattern as a sequence of UTF-8
 * characters. The results of scanning invalid UTF-8 sequences with a Hyperscan
 * library that has been compiled with one or more patterns using this flag are
 * undefined.
 */
pub const HS_FLAG_UTF8: u32 = 32;

/**
 * Compile flag: Enable Unicode property support for this expression.
 *
 * This flag instructs Hyperscan to use Unicode properties, rather than the
 * default ASCII interpretations, for character mnemonics like `\w` and `\s` as
 * well as the POSIX character classes. It is only meaningful in conjunction
 * with @ref HS_FLAG_UTF8.
 */
pub const HS_FLAG_UCP: u32 = 64;

/**
 * Compile flag: Enable prefiltering mode for this expression.
 *
 * This flag instructs Hyperscan to compile an "approximate" version of this
 * pattern for use in a prefiltering application, even if Hyperscan does not
 * support the pattern in normal operation.
 *
 * The set of matches returned when this flag is used is guaranteed to be a
 * superset of the matches specified by the non-prefiltering expression.
 *
 * If the pattern contains pattern constructs not supported by Hyperscan (such
 * as zero-width assertions, back-references or conditional references) these
 * constructs will be replaced internally with broader constructs that may
 * match more often.
 *
 * Furthermore, in prefiltering mode Hyperscan may simplify a pattern that
 * would otherwise return a "Pattern too large" error at compile time, or for
 * performance reasons (subject to the matching guarantee above).
 *
 * It is generally expected that the application will subsequently confirm
 * prefilter matches with another regular expression matcher that can provide
 * exact matches for the pattern.
 *
 * Note: The use of this flag in combination with @ref HS_FLAG_SOM_LEFTMOST
 * is not currently supported.
 */
pub const HS_FLAG_PREFILTER: u32 = 128;

/**
 * Compile flag: Enable leftmost start of match reporting.
 *
 * This flag instructs Hyperscan to report the leftmost possible start of match
 * offset when a match is reported for this expression. (By default, no start
 * of match is returned.)
 *
 * Enabling this behaviour may reduce performance and increase stream state
 * requirements in streaming mode.
 */
pub const HS_FLAG_SOM_LEFTMOST: u32 = 256;

impl<T: Type> RawDatabase<T> {
    pub fn compile(expression: &str, flags: u32) -> Result<RawDatabase<T>, Error> {
        let mut db: *mut hs_database_t = ptr::null_mut();
        let platform: *const hs_platform_info_t = ptr::null();
        let mut err: *mut hs_compile_error_t = ptr::null_mut();

        unsafe {
            check_compile_error!(hs_compile(expression.as_ptr() as *const i8,
                                            flags,
                                            T::mode(),
                                            platform,
                                            &mut db,
                                            &mut err),
                                 err);
        }

        Result::Ok(RawDatabase::new(db))
    }
}

pub trait DatabaseBuilder {
    fn build<T: Type>(&self) -> Result<RawDatabase<T>, Error>;
}

struct Pattern {
    expression: String,
    flags: u32,
    id: usize,
}

type Patterns = Vec<Pattern>;

#[macro_export]
macro_rules! pattern {
    ( $expr:expr ) => (
        $crate::compile::Pattern{expression: $crate::std::convert::From::from($expr), flags: 0, id: 0}
    );
    ( $expr:expr, flags => $flags:expr ) => (
        $crate::compile::Pattern{expression: $crate::std::convert::From::from($expr), flags: $flags, id: 0}
    );
    ( $expr:expr, flags => $flags:expr, id => $id:expr ) => (
        $crate::compile::Pattern{expression: $crate::std::convert::From::from($expr), flags: $flags, id: $id}
    );
}

#[macro_export]
macro_rules! patterns {
    ( [ $( $expr:expr ), * ] ) => {
        {
            let mut v = Vec::new();
            $(
                let id = v.len() + 1;

                v.push(pattern!($expr, flags => 0, id => id));
            )*
            v
        }
    };
    ( [ $( $expr:expr ), * ], flags => $flags:expr ) => {
        {
            let mut v = Vec::new();
            $(
                let id = v.len() + 1;

                v.push(pattern!($expr, flags => $flags, id => id));
            )*
            v
        }
    };
}

impl DatabaseBuilder for Pattern {
    fn build<T: Type>(&self) -> Result<RawDatabase<T>, Error> {
        RawDatabase::compile(&self.expression, self.flags)
    }
}

impl DatabaseBuilder for Patterns {
    fn build<T: Type>(&self) -> Result<RawDatabase<T>, Error> {
        let mut expressions = Vec::new();
        let mut flags = Vec::new();
        let mut ids = Vec::new();

        for pattern in self {
            expressions.push(pattern.expression.as_str().as_ptr());
            flags.push(pattern.flags);
            ids.push(pattern.id as u32);
        }

        let platform: *const hs_platform_info_t = ptr::null();
        let mut db: *mut hs_database_t = ptr::null_mut();
        let mut err: *mut hs_compile_error_t = ptr::null_mut();

        unsafe {
            check_compile_error!(hs_compile_multi(mem::transmute(expressions.as_slice().as_ptr()),
                                                  flags.as_slice().as_ptr(),
                                                  ids.as_slice().as_ptr(),
                                                  self.len() as u32,
                                                  T::mode(),
                                                  platform,
                                                  &mut db,
                                                  &mut err),
                                 err);
        }

        Result::Ok(RawDatabase::new(db))
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;

    use super::super::common::BlockDatabase;

    use super::super::common::tests::{validate_database, validate_database_with_size};

    use super::*;

    const DATABASE_SIZE: usize = 2984;

    #[test]
    fn test_database_compile() {
        let db = BlockDatabase::compile("test", 0).unwrap();

        assert!(*db != ptr::null_mut());

        validate_database(&db);
    }

    #[test]
    fn test_pattern_build() {
        let p = pattern!{"test"};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, 0);
        assert_eq!(p.id, 0);

        let b = &p;

        let db: BlockDatabase = b.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_pattern_build_with_flags() {
        let p = pattern!{"test", flags => HS_FLAG_CASELESS};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, HS_FLAG_CASELESS);
        assert_eq!(p.id, 0);

        let b = &p;

        let db: BlockDatabase = b.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_patterns_build() {
        let p = patterns!(["test", "foo", "bar"]);

        let db: BlockDatabase = p.build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }

    #[test]
    fn test_patterns_build_with_flags() {
        let p = patterns!(["test", "foo", "bar"], flags => HS_FLAG_CASELESS);

        let db: BlockDatabase = p.build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }
}
