use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

use bitflags::bitflags;
use derive_more::{Deref, DerefMut, From, Index, IndexMut, Into, IntoIterator};

use crate::{compile::SomHorizon, ffi, Error, Result};

bitflags! {
    /// Literal flags
    #[derive(Default)]
    pub struct Flags: u32 {
        /// Matching will be performed case-insensitively.
        const CASELESS = ffi::HS_FLAG_CASELESS;
        /// `^` and `$` anchors match any newlines in data.
        const MULTILINE = ffi::HS_FLAG_MULTILINE;
        /// Only one match will be generated for the expression per stream.
        const SINGLEMATCH = ffi::HS_FLAG_SINGLEMATCH;
        /// Report the leftmost start of match offset when a match is found.
        const SOM_LEFTMOST = ffi::HS_FLAG_SOM_LEFTMOST;
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
                'H' => flags |= Flags::SINGLEMATCH,
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
        if self.contains(Flags::SINGLEMATCH) {
            write!(f, "H")?
        }
        Ok(())
    }
}

/// The pattern with pure literal expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Literal {
    /// The expression to parse.
    pub expression: String,
    /// Flags which modify the behaviour of the expression.
    pub flags: Flags,
    /// ID number to be associated with the corresponding literal in the expressions array.
    pub id: Option<usize>,
    /// The precision to track start of match offsets in stream state.
    pub som: Option<SomHorizon>,
}

impl Literal {
    /// Construct a literal with expression.
    pub fn new<S: Into<String>>(expr: S) -> Result<Literal> {
        Ok(Literal {
            expression: expr.into(),
            flags: Flags::empty(),
            id: None,
            som: None,
        })
    }

    /// Construct a literal with expression and flags.
    pub fn with_flags<S: Into<String>>(expr: S, flags: Flags) -> Result<Literal> {
        Ok(Literal {
            expression: expr.into(),
            flags,
            id: None,
            som: None,
        })
    }

    /// Set case-insensitive matching.
    pub fn caseless(mut self) -> Self {
        self.flags |= Flags::CASELESS;
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

    /// Report the leftmost start of match offset when a match is found.
    pub fn left_most(mut self) -> Self {
        self.flags |= Flags::SOM_LEFTMOST;
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

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = self.id {
            write!(f, "{}:", id)?;
        }

        if self.id.is_some() || !self.flags.is_empty() {
            write!(f, "/{}/", self.expression)?;
        } else {
            write!(f, "{}", self.expression)?;
        }

        if !self.flags.is_empty() {
            write!(f, "{}", self.flags)?;
        }

        Ok(())
    }
}

impl FromStr for Literal {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let (id, expr) = match s.find(':') {
            Some(off) => (Some(s[..off].parse()?), &s[off + 1..]),
            None => (None, s),
        };

        let literal = match (expr.starts_with('/'), expr.rfind('/')) {
            (true, Some(end)) if end > 0 => Literal {
                expression: expr[1..end].into(),
                flags: expr[end + 1..].parse()?,
                id,
                som: None,
            },

            _ => Literal {
                expression: expr.into(),
                flags: Flags::empty(),
                id,
                som: None,
            },
        };

        Ok(literal)
    }
}

/// Vec of `Literal`
#[repr(transparent)]
#[derive(Clone, Debug, Deref, DerefMut, From, Index, IndexMut, Into, IntoIterator)]
#[deref(forward)]
#[deref_mut(forward)]
pub struct Literals(Vec<Literal>);

impl FromIterator<Literal> for Literals {
    fn from_iter<T: IntoIterator<Item = Literal>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl FromStr for Literals {
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

impl Literals {
    pub(crate) fn som(&self) -> Option<SomHorizon> {
        if self
            .iter()
            .any(|Literal { flags, .. }| flags.contains(Flags::SOM_LEFTMOST))
        {
            self.iter()
                .flat_map(|&Literal { som, .. }| som)
                .max()
                .or(Some(SomHorizon::Medium))
        } else {
            None
        }
    }
}

/// Define `Literal` with flags
#[macro_export]
macro_rules! literal {
    ( $expr:expr ) => {{
        literal! { $expr ; $crate::LiteralFlags::default() }
    }};
    ( $expr:expr ; $( $flag:ident )|* ) => {{
        literal! { $expr ; $( $crate::LiteralFlags:: $flag )|* }
    }};
    ( $expr:expr ; $flags:expr ) => {{
        $crate::Literal {
            expression: $expr.into(),
            flags: $flags,
            id: None,
            som: None,
        }
    }};
    ( $id:literal => $expr:expr ; $( $flag:ident )|* ) => {{
        literal! { $id => $expr ; $( $crate::LiteralFlags:: $flag )|* }
    }};
    ( $id:literal => $expr:expr ; $flags:expr ) => {{
        $crate::Literal {
            expression: $expr.into(),
            flags: $flags,
            id: Some($id),
            som: None,
        }
    }};
}

/// Define multi `Literal` with flags and ID
#[macro_export]
macro_rules! literals {
    ( $( $expr:expr ),* ) => {
        Literals(vec![ $( literal! { $expr } ),* ])
    };
    ( $( $expr:expr ),* ; $( $flag:ident )|* ) => {
        literals! { $( $expr ),*; $( $crate::LiteralFlags:: $flag )|* }
    };
    ( $( $expr:expr ),* ; $flags:expr ) => {{
        Literals(vec![ $( literal! { $expr ; $flags } ),* ])
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
        let flags = Flags::CASELESS;

        assert_eq!(flags.to_string(), "i");

        assert_eq!("im".parse::<Flags>().unwrap(), flags | Flags::MULTILINE);
        assert!("test".parse::<Flags>().is_err());
    }

    #[test]
    fn test_literal() {
        let p: Literal = "test".parse().unwrap();

        assert_eq!(p, literal! { "test" });
        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p: Literal = "/test/".parse().unwrap();

        assert_eq!(p, literal! { "test" });
        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p: Literal = "/test/i".parse().unwrap();

        assert_eq!(p, literal! { "test"; CASELESS });
        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);

        let p: Literal = "3:/test/i".parse().unwrap();

        assert_eq!(p, literal! { 3 => "test"; CASELESS });
        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, Some(3));

        let p: Literal = "test/i".parse().unwrap();

        assert_eq!(p, literal! { "test/i" });
        assert_eq!(p.expression, "test/i");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let p: Literal = "/t/e/s/t/i".parse().unwrap();

        assert_eq!(p, literal! { "t/e/s/t"; CASELESS });
        assert_eq!(p.expression, "t/e/s/t");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);
    }

    #[test]
    fn test_pattern_build() {
        let p = &literal! {"test"};

        assert_eq!(p.expression, "test");
        assert!(p.flags.is_empty());
        assert_eq!(p.id, None);

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_pattern_build_with_flags() {
        let p = &literal! {"test"; CASELESS};

        assert_eq!(p.expression, "test");
        assert_eq!(p.flags, Flags::CASELESS);
        assert_eq!(p.id, None);

        let db: BlockDatabase = p.build().unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_patterns_build() {
        let db: BlockDatabase = literals!("test", "foo", "bar").build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }

    #[test]
    fn test_patterns_build_with_flags() {
        let db: BlockDatabase = literals!("test", "foo", "bar"; CASELESS).build().unwrap();

        validate_database_with_size(&db, DATABASE_SIZE);
    }
}
