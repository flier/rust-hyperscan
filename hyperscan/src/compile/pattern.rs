use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

use anyhow::{bail, Error, Result};
use bitflags::bitflags;
use derive_more::{Deref, DerefMut, From, Index, IndexMut, Into, IntoIterator};

use crate::ffi;

bitflags! {
    /// Pattern flags
    pub struct Flags: u32 {
        /// Set case-insensitive matching.
        const CASELESS = ffi::HS_FLAG_CASELESS;
        /// Matching a `.` will not exclude newlines.
        const DOTALL = ffi::HS_FLAG_DOTALL;
        /// Set multi-line anchoring.
        const MULTILINE = ffi::HS_FLAG_MULTILINE;
        /// Set single-match only mode.
        const SINGLEMATCH = ffi::HS_FLAG_SINGLEMATCH;
        /// Allow expressions that can match against empty buffers.
        const ALLOWEMPTY = ffi::HS_FLAG_ALLOWEMPTY;
        /// Enable UTF-8 mode for this expression.
        const UTF8 = ffi::HS_FLAG_UTF8;
        /// Enable Unicode property support for this expression.
        const UCP = ffi::HS_FLAG_UCP;
        /// Enable prefiltering mode for this expression.
        const PREFILTER = ffi::HS_FLAG_PREFILTER;
        /// Enable leftmost start of match reporting.
        const SOM_LEFTMOST = ffi::HS_FLAG_SOM_LEFTMOST;
        /// Logical combination.
        const COMBINATION = ffi::HS_FLAG_COMBINATION;
        /// Don't do any match reporting.
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
                    bail!("invalid pattern flag: {}", c);
                }
            }
        }

        Ok(flags)
    }
}

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
            flags |= u64::from(ffi::HS_EXT_FLAG_MIN_OFFSET);
        }
        if ext.max_offset.is_some() {
            flags |= u64::from(ffi::HS_EXT_FLAG_MAX_OFFSET);
        }
        if ext.min_length.is_some() {
            flags |= u64::from(ffi::HS_EXT_FLAG_MIN_LENGTH);
        }
        if ext.edit_distance.is_some() {
            flags |= u64::from(ffi::HS_EXT_FLAG_EDIT_DISTANCE);
        }
        if ext.hamming_distance.is_some() {
            flags |= u64::from(ffi::HS_EXT_FLAG_HAMMING_DISTANCE);
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

/// Defines the precision to track start of match offsets in stream state.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SomHorizon {
    /// use full precision to track start of match offsets in stream state.
    ///
    /// This mode will use the most stream state per pattern,
    /// but will always return an accurate start of match offset
    /// regardless of how far back in the past it was found.
    Large = ffi::HS_MODE_SOM_HORIZON_LARGE,
    /// use medium precision to track start of match offsets in stream state.
    ///
    /// This mode will use less stream state than @ref HS_MODE_SOM_HORIZON_LARGE and
    /// will limit start of match accuracy to offsets
    /// within 2^32 bytes of the end of match offset reported.
    Medium = ffi::HS_MODE_SOM_HORIZON_MEDIUM,
    /// use limited precision to track start of match offsets in stream state.
    ///
    /// This mode will use less stream state than `SomHorizon::Large` and
    /// will limit start of match accuracy to offsets
    /// within 2^16 bytes of the end of match offset reported.
    Small = ffi::HS_MODE_SOM_HORIZON_SMALL,
}

/// The pattern with basic regular expression.
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    /// The expression to parse.
    pub expression: String,
    /// Flags which modify the behaviour of the expression.
    pub flags: Flags,
    /// ID number to be associated with the corresponding pattern in the expressions array.
    pub id: Option<usize>,
    /// Extended behaviour for this pattern
    pub ext: Ext,
    /// The precision to track start of match offsets in stream state.
    pub som: Option<SomHorizon>,
}

impl Pattern {
    /// Construct a pattern with expression.
    pub fn new<S: Into<String>>(expr: S) -> Result<Pattern> {
        Ok(Pattern {
            expression: expr.into(),
            flags: Flags::empty(),
            id: None,
            ext: Ext::default(),
            som: None,
        })
    }

    /// Construct a pattern with expression and flags.
    pub fn with_flags<S: Into<String>>(expr: S, flags: Flags) -> Result<Pattern> {
        Ok(Pattern {
            expression: expr.into(),
            flags,
            id: None,
            ext: Ext::default(),
            som: None,
        })
    }

    /// Parse a basic regular expression to a pattern.
    pub fn parse<S: AsRef<str>>(s: S) -> Result<Pattern> {
        let s = s.as_ref();
        let (id, expr) = match s.find(":/") {
            Some(off) => (Some(s[..off].parse()?), &s[off + 1..]),
            None => (None, s),
        };

        let pattern = match (expr.starts_with('/'), expr.rfind('/')) {
            (true, Some(end)) if end > 0 => Pattern {
                expression: expr[1..end].into(),
                flags: expr[end + 1..].parse()?,
                id,
                ext: Ext::default(),
                som: None,
            },

            _ => Pattern {
                expression: expr.into(),
                flags: Flags::empty(),
                id,
                ext: Ext::default(),
                som: None,
            },
        };

        debug!("pattern `{}` parsed to `{}`", s, pattern);

        Ok(pattern)
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = self.id {
            write!(f, "{}:", id)?;
        }

        let expr = regex_syntax::escape(self.expression.as_str());

        if self.id.is_some() || !self.flags.is_empty() {
            write!(f, "/{}/", expr)?;
        } else {
            write!(f, "{}", expr)?;
        }

        if !self.flags.is_empty() {
            write!(f, "{}", self.flags)?;
        }

        Ok(())
    }
}

impl FromStr for Pattern {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Pattern::parse(s)
    }
}

/// Vec of `Pattern`
#[repr(transparent)]
#[derive(Clone, Debug, Deref, DerefMut, From, Index, IndexMut, Into, IntoIterator)]
#[deref(forward)]
#[deref_mut(forward)]
pub struct Patterns(Vec<Pattern>);

impl FromIterator<Pattern> for Patterns {
    fn from_iter<T: IntoIterator<Item = Pattern>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl FromStr for Patterns {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.lines()
            .flat_map(|line| {
                let line = line.trim();

                if line.is_empty() || line.starts_with('#') {
                    None
                } else {
                    Some(line.parse())
                }
            })
            .collect::<Result<Vec<_>>>()
            .map(Self)
    }
}

/// Define `Pattern` with flags
#[macro_export]
macro_rules! pattern {
    ( $expr:expr ) => {{
        pattern! { $expr ; $crate::CompileFlags::default() }
    }};
    ( $expr:expr ; $( $flag:ident )|* ) => {{
        pattern! { $expr ; $( $crate::CompileFlags:: $flag )|* }
    }};
    ( $expr:expr ; $flags:expr ) => {{
        $crate::Pattern {
            expression: $expr.into(),
            flags: $flags,
            id: None,
            ext: $crate::ExpressionExt::default(),
            som: None,
        }
    }};
    ( $id:literal => $expr:expr ; $( $flag:ident )|* ) => {{
        pattern! { $id => $expr ; $( $crate::CompileFlags:: $flag )|* }
    }};
    ( $id:literal => $expr:expr ; $flags:expr ) => {{
        $crate::Pattern {
            expression: $expr.into(),
            flags: $flags,
            id: Some($id),
            ext: $crate::ExpressionExt::default(),
            som: None,
        }
    }};
}

/// Define multi `Pattern` with flags and ID
#[macro_export]
macro_rules! patterns {
    ( $( $expr:expr ),* ) => {
        Patterns(vec![ $( pattern! { $expr } ),* ])
    };
    ( $( $expr:expr ),* ; $( $flag:ident )|* ) => {
        patterns! { $( $expr ),*; $( $crate::CompileFlags:: $flag )|* }
    };
    ( $( $expr:expr ),* ; $flags:expr ) => {{
        Patterns(vec![ $( pattern! { $expr ; $flags } ),* ])
    }};
}

#[cfg(test)]
mod tests {
    use crate::common::tests::*;
    use crate::prelude::*;

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

        assert_eq!(p, pattern! { "test" });
        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p = Pattern::parse("/test/").unwrap();

        assert_eq!(p, pattern! { "test" });
        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p = Pattern::parse("/test/i").unwrap();

        assert_eq!(p, pattern! { "test"; CASELESS });
        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);

        let p = Pattern::parse("3:/test/i").unwrap();

        assert_eq!(p, pattern! { 3 => "test"; CASELESS });
        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, Some(3));

        let p = Pattern::parse("test/i").unwrap();

        assert_eq!(p, pattern! { "test/i" });
        assert_eq!(p.expression, "test/i");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p = Pattern::parse("/t/e/s/t/i").unwrap();

        assert_eq!(p, pattern! { "t/e/s/t"; CASELESS });
        assert_eq!(p.expression, "t/e/s/t");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);
    }

    #[test]
    fn test_pattern_build() {
        let _ = pretty_env_logger::try_init();

        let p = &pattern! {"test"};

        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

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

        let p = &pattern! {"test"; CASELESS};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_patterns_build() {
        let _ = pretty_env_logger::try_init();

        let db: BlockDatabase = patterns!("test", "foo", "bar").build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }

    #[test]
    fn test_patterns_build_with_flags() {
        let _ = pretty_env_logger::try_init();

        let db: BlockDatabase = patterns!("test", "foo", "bar"; CASELESS | DOTALL).build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }
}
