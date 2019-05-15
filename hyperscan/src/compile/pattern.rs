use core::fmt;
use core::str::FromStr;

use bitflags::bitflags;
use failure::{bail, Error};

bitflags! {
    pub struct Flags: u32 {
        const CASELESS = ffi::HS_FLAG_CASELESS;
        const DOTALL = ffi::HS_FLAG_DOTALL;
        const MULTILINE = ffi::HS_FLAG_MULTILINE;
        const SINGLEMATCH = ffi::HS_FLAG_SINGLEMATCH;
        const ALLOWEMPTY = ffi::HS_FLAG_ALLOWEMPTY;
        const UTF8 = ffi::HS_FLAG_UTF8;
        const UCP = ffi::HS_FLAG_UCP;
        const PREFILTER = ffi::HS_FLAG_PREFILTER;
        const SOM_LEFTMOST = ffi::HS_FLAG_SOM_LEFTMOST;
        const COMBINATION = ffi::HS_FLAG_COMBINATION;
        const QUIET = ffi::HS_FLAG_QUIET;
    }
}

impl Default for Flags {
    fn default() -> Self {
        Flags::empty()
    }
}

impl FromStr for Flags {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut flags = Flags::empty();

        for c in s.chars() {
            match c {
                'i' => flags |= Flags::CASELESS,
                'm' => flags |= Flags::MULTILINE,
                's' => flags |= Flags::DOTALL,
                'H' => flags |= Flags::SINGLEMATCH,
                'V' => flags |= Flags::ALLOWEMPTY,
                '8' => flags |= Flags::UTF8,
                'W' => flags |= Flags::UCP,
                'C' => flags |= Flags::COMBINATION,
                'Q' => flags |= Flags::QUIET,
                _ => {
                    bail!("invalid compile flag: {}", c);
                }
            }
        }

        Ok(flags)
    }
}

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.contains(Flags::CASELESS) {
            write!(f, "i")?
        }
        if self.contains(Flags::MULTILINE) {
            write!(f, "m")?
        }
        if self.contains(Flags::DOTALL) {
            write!(f, "s")?
        }
        if self.contains(Flags::SINGLEMATCH) {
            write!(f, "H")?
        }
        if self.contains(Flags::ALLOWEMPTY) {
            write!(f, "V")?
        }
        if self.contains(Flags::UTF8) {
            write!(f, "8")?
        }
        if self.contains(Flags::UCP) {
            write!(f, "W")?
        }
        if self.contains(Flags::COMBINATION) {
            write!(f, "C")?
        }
        if self.contains(Flags::QUIET) {
            write!(f, "Q")?
        }
        Ok(())
    }
}

/// A structure containing additional parameters related to an expression.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ext {
    /// The minimum end offset in the data stream at which this expression should match successfully.
    pub min_offset: Option<u64>,

    /// The maximum end offset in the data stream at which this expression should match successfully.
    pub max_offset: Option<u64>,

    /// The minimum match length (from start to end) required to successfully match this expression.
    pub min_length: Option<u64>,

    /// Allow patterns to approximately match within this edit distance.
    pub edit_distance: Option<u32>,

    /// Allow patterns to approximately match within this Hamming distance.
    pub hamming_distance: Option<u32>,
}

impl From<Ext> for ffi::hs_expr_ext_t {
    fn from(ext: Ext) -> Self {
        let mut flags = 0;

        if ext.min_offset.is_some() {
            flags |= ffi::HS_EXT_FLAG_MIN_OFFSET as u64;
        }
        if ext.max_offset.is_some() {
            flags |= ffi::HS_EXT_FLAG_MAX_OFFSET as u64;
        }
        if ext.min_length.is_some() {
            flags |= ffi::HS_EXT_FLAG_MIN_LENGTH as u64;
        }
        if ext.edit_distance.is_some() {
            flags |= ffi::HS_EXT_FLAG_EDIT_DISTANCE as u64;
        }
        if ext.hamming_distance.is_some() {
            flags |= ffi::HS_EXT_FLAG_HAMMING_DISTANCE as u64;
        }

        ffi::hs_expr_ext_t {
            flags,
            min_offset: ext.min_offset.unwrap_or_default(),
            max_offset: ext.max_offset.unwrap_or_default(),
            min_length: ext.min_length.unwrap_or_default(),
            edit_distance: ext.edit_distance.unwrap_or_default(),
            hamming_distance: ext.hamming_distance.unwrap_or_default(),
        }
    }
}

/// Pattern that has matched.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// The expression to parse.
    pub expression: String,
    /// Flags which modify the behaviour of the expression.
    pub flags: Flags,
    /// ID number to be associated with the corresponding pattern in the expressions array.
    pub id: usize,
    /// Extended behaviour for this pattern
    pub ext: Ext,
}

impl Pattern {
    pub fn parse(s: &str) -> Result<Pattern, Error> {
        unsafe {
            let (id, expr) = match s.find(':') {
                Some(off) => (s.get_unchecked(0..off).parse()?, s.get_unchecked(off + 1..s.len())),
                None => (0, s),
            };

            let pattern = match (expr.starts_with('/'), expr.rfind('/')) {
                (true, Some(end)) if end > 0 => Pattern {
                    expression: expr.get_unchecked(1..end).into(),
                    flags: expr.get_unchecked(end + 1..expr.len()).parse()?,
                    id,
                    ext: Ext::default(),
                },

                _ => Pattern {
                    expression: String::from(expr),
                    flags: Flags::empty(),
                    id,
                    ext: Ext::default(),
                },
            };

            debug!("pattern `{}` parsed to `{}`", s, pattern);

            Ok(pattern)
        }
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

/// Vec of `Pattern`
pub type Patterns = Vec<Pattern>;

/// Define `Pattern` with flags
#[macro_export]
macro_rules! pattern {
    ($expr:expr) => {{
        pattern!($expr, flags => Default::default(), id => 0)
    }};
    ($expr:expr, flags => $flags:expr) => {{
        pattern!($expr, flags => $flags, id => 0)
    }};
    ($expr:expr, flags => $flags:expr, id => $id:expr) => {{
        $crate::Pattern {
            expression: $expr.into(),
            flags: $flags,
            id: $id,
            ext: $crate::ExpressionExt::default(),
        }
    }};
}

/// Define multi `Pattern` with flags and ID
#[macro_export]
macro_rules! patterns {
    ( [ $( $expr:expr ), * ] ) => {{
        patterns!([ $( $expr ), * ], flags => Default::default())
    }};
    ( [ $( $expr:expr ), * ], flags => $flags:expr ) => {{
        let mut v = Vec::new();
        $(
            v.push(pattern!{$expr, flags => $flags, id => v.len() + 1});
        )*

        v
    }};
}

#[cfg(test)]
mod tests {
    use crate::common::tests::*;
    use crate::common::BlockDatabase;
    use crate::compile::Builder;

    use super::*;

    const DATABASE_SIZE: usize = 2664;

    #[test]
    fn test_compile_flags() {
        let _ = pretty_env_logger::try_init();

        let flags = Flags::CASELESS | Flags::DOTALL;

        assert_eq!(flags.to_string(), "is");

        assert_eq!("ism".parse::<Flags>().unwrap(), flags | Flags::MULTILINE);
        assert!("test".parse::<Flags>().is_err());
    }

    #[test]
    fn test_pattern() {
        let _ = pretty_env_logger::try_init();

        let p = Pattern::parse("test").unwrap();

        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, 0);

        let p = Pattern::parse("/test/").unwrap();

        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, 0);

        let p = Pattern::parse("/test/i").unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, 0);

        let p = Pattern::parse("3:/test/i").unwrap();

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, 3);

        let p = Pattern::parse("test/i").unwrap();

        assert_eq!(p.expression, "test/i");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, 0);

        let p = Pattern::parse("/t/e/s/t/i").unwrap();

        assert_eq!(p.expression, "t/e/s/t");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, 0);
    }

    #[test]
    fn test_pattern_build() {
        let _ = pretty_env_logger::try_init();

        let p = &pattern! {"test"};

        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
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

        let p = &pattern! {"test", flags => Flags::CASELESS};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
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

        let db: BlockDatabase = patterns!(["test", "foo", "bar"], flags => Flags::CASELESS|Flags::DOTALL)
            .build()
            .unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }
}
