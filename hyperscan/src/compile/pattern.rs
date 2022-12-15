use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

use bitflags::bitflags;
use derive_more::{Deref, DerefMut, From, Index, IndexMut, Into, IntoIterator};

use crate::{
    compile::ExprExt,
    error::{Error, Result},
    ffi,
};

bitflags! {
    /// Pattern flags
    #[derive(Default)]
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
        #[cfg(feature = "v5")]
        const COMBINATION = ffi::HS_FLAG_COMBINATION;
        /// Don't do any match reporting.
        #[cfg(feature = "v5")]
        const QUIET = ffi::HS_FLAG_QUIET;
    }
}

impl FromStr for Flags {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
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
                'P' => flags |= Flags::PREFILTER,
                'L' => flags |= Flags::SOM_LEFTMOST,
                #[cfg(feature = "v5")]
                'C' => flags |= Flags::COMBINATION,
                #[cfg(feature = "v5")]
                'Q' => flags |= Flags::QUIET,
                _ => return Err(Error::InvalidFlag(c)),
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
        if self.contains(Flags::PREFILTER) {
            write!(f, "P")?
        }
        if self.contains(Flags::SOM_LEFTMOST) {
            write!(f, "L")?
        }
        #[cfg(feature = "v5")]
        if self.contains(Flags::COMBINATION) {
            write!(f, "C")?
        }
        #[cfg(feature = "v5")]
        if self.contains(Flags::QUIET) {
            write!(f, "Q")?
        }
        Ok(())
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    /// The expression to parse.
    pub expression: String,
    /// Flags which modify the behaviour of the expression.
    pub flags: Flags,
    /// ID number to be associated with the corresponding pattern in the expressions array.
    pub id: Option<usize>,
    /// Extended behaviour for this pattern
    pub ext: ExprExt,
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
            ext: ExprExt::default(),
            som: None,
        })
    }

    /// Construct a pattern with expression and flags.
    pub fn with_flags<S: Into<String>>(expr: S, flags: Flags) -> Result<Pattern> {
        Ok(Pattern {
            expression: expr.into(),
            flags,
            id: None,
            ext: ExprExt::default(),
            som: None,
        })
    }

    /// Set case-insensitive matching.
    pub fn caseless(mut self) -> Self {
        self.flags |= Flags::CASELESS;
        self
    }

    /// Matching a `.` will not exclude newlines.
    pub fn dot_all(mut self) -> Self {
        self.flags |= Flags::DOTALL;
        self
    }

    /// Set multi-line anchoring.
    pub fn multi_line(mut self) -> Self {
        self.flags |= Flags::MULTILINE;
        self
    }

    /// Set single-match only mode.
    pub fn single_match(mut self) -> Self {
        self.flags |= Flags::SINGLEMATCH;
        self
    }

    /// Allow expressions that can match against empty buffers.
    pub fn allow_empty(mut self) -> Self {
        self.flags |= Flags::ALLOWEMPTY;
        self
    }

    /// Enable UTF-8 mode for this expression.
    pub fn utf8(mut self) -> Self {
        self.flags |= Flags::UTF8;
        self
    }

    /// Enable Unicode property support for this expression.
    pub fn ucp(mut self) -> Self {
        self.flags |= Flags::UCP;
        self
    }

    /// Enable prefiltering mode for this expression.
    pub fn prefilter(mut self) -> Self {
        self.flags |= Flags::PREFILTER;
        self
    }

    /// Report the leftmost start of match offset when a match is found.
    pub fn left_most(mut self) -> Self {
        self.flags |= Flags::SOM_LEFTMOST;
        self
    }

    /// Logical combination.
    #[cfg(feature = "v5")]
    pub fn combination(mut self) -> Self {
        self.flags |= Flags::COMBINATION;
        self
    }

    /// Don't do any match reporting.
    #[cfg(feature = "v5")]
    pub fn quiet(mut self) -> Self {
        self.flags |= Flags::QUIET;
        self
    }

    pub(crate) fn som(&self) -> Option<SomHorizon> {
        if self.flags.contains(Flags::SOM_LEFTMOST) {
            self.som.or(Some(SomHorizon::Medium))
        } else {
            None
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = self.id {
            write!(f, "{}:", id)?;
        }

        if self.id.is_some() || !self.flags.is_empty() || !self.ext.is_empty() {
            write!(f, "/{}/", self.expression)?;
        } else {
            write!(f, "{}", self.expression)?;
        }

        if !self.flags.is_empty() {
            write!(f, "{}", self.flags)?;
        }
        if !self.ext.is_empty() {
            write!(f, "{}", self.ext)?;
        }

        Ok(())
    }
}

impl FromStr for Pattern {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (id, expr) = match s.find(":/") {
            Some(off) => (Some(s[..off].parse()?), &s[off + 1..]),
            None => (None, s),
        };

        match (expr.starts_with('/'), expr.rfind('/')) {
            (true, Some(end)) if end > 0 => {
                let (expr, remaining) = (&expr[1..end], &expr[end + 1..]);
                let (flags, ext) = match (remaining.ends_with('}'), remaining.rfind('{')) {
                    (true, Some(start)) => {
                        let (flags, ext) = remaining.split_at(start);

                        (flags.parse()?, ext.parse()?)
                    }
                    _ => (remaining.parse()?, ExprExt::default()),
                };

                Ok(Pattern {
                    expression: expr.into(),
                    flags,
                    id,
                    ext,
                    som: None,
                })
            }

            _ => Ok(Pattern {
                expression: expr.into(),
                flags: Flags::empty(),
                id,
                ext: ExprExt::default(),
                som: None,
            }),
        }
    }
}

/// Vec of `Pattern`
#[repr(transparent)]
#[derive(Clone, Debug, Deref, DerefMut, From, Index, IndexMut, Into, IntoIterator)]
#[deref(forward)]
#[deref_mut(forward)]
pub struct Patterns(pub Vec<Pattern>);

impl FromIterator<Pattern> for Patterns {
    fn from_iter<T: IntoIterator<Item = Pattern>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl FromStr for Patterns {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
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

impl Patterns {
    pub(crate) fn som(&self) -> Option<SomHorizon> {
        if self
            .iter()
            .any(|Pattern { flags, .. }| flags.contains(Flags::SOM_LEFTMOST))
        {
            self.iter()
                .flat_map(|&Pattern { som, .. }| som)
                .max()
                .or(Some(SomHorizon::Medium))
        } else {
            None
        }
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
        let flags = Flags::CASELESS | Flags::DOTALL;

        assert_eq!(flags.to_string(), "is");

        assert_eq!("ism".parse::<Flags>().unwrap(), flags | Flags::MULTILINE);
        assert!("test".parse::<Flags>().is_err());
    }

    #[test]
    fn test_pattern() {
        let p: Pattern = "test".parse().unwrap();

        assert_eq!(p, pattern! { "test" });
        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p: Pattern = "/test/".parse().unwrap();

        assert_eq!(p, pattern! { "test" });
        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p: Pattern = "/test/i".parse().unwrap();

        assert_eq!(p, pattern! { "test"; CASELESS });
        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);

        let p: Pattern = "3:/test/i".parse().unwrap();

        assert_eq!(p, pattern! { 3 => "test"; CASELESS });
        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, Some(3));

        let s = r#"1:/hatstand.*teakettle/s{min_offset=50,max_offset=100}"#;
        let p: Pattern = s.parse().unwrap();

        assert_eq!(p, {
            let mut p = pattern! { 1 => "hatstand.*teakettle"; DOTALL };
            p.ext.set_min_offset(50);
            p.ext.set_max_offset(100);
            p
        });
        assert_eq!(p.expression, "hatstand.*teakettle");
        assert_eq!(p.flags, Flags::DOTALL);
        assert_eq!(p.id, Some(1));
        assert_eq!(p.ext.min_offset().unwrap(), 50);
        assert_eq!(p.ext.max_offset().unwrap(), 100);
        assert_eq!(p.to_string(), s);

        let p: Pattern = "test/i".parse().unwrap();

        assert_eq!(p, pattern! { "test/i" });
        assert_eq!(p.expression, "test/i");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p: Pattern = "/t/e/s/t/i".parse().unwrap();

        assert_eq!(p, pattern! { "t/e/s/t"; CASELESS });
        assert_eq!(p.expression, "t/e/s/t");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);
    }

    #[test]
    fn test_pattern_build() {
        let p = &pattern! {"test"};

        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let info = p.info().unwrap();

        assert_eq!(info.min_width, 4);
        assert_eq!(info.max_width, 4);
        assert!(!info.unordered_matches());
        assert!(!info.matches_at_eod());
        assert!(!info.matches_only_at_eod());

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_pattern_build_with_flags() {
        let p = &pattern! {"test"; CASELESS};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_patterns_build() {
        let db: BlockDatabase = patterns!("test", "foo", "bar").build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }

    #[test]
    fn test_patterns_build_with_flags() {
        let db: BlockDatabase = patterns!("test", "foo", "bar"; CASELESS | DOTALL).build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }
}
