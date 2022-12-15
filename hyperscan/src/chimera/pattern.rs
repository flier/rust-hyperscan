use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

use bitflags::bitflags;
use derive_more::{Deref, DerefMut, From, Index, IndexMut, Into, IntoIterator};

use crate::{chimera::ffi, Error};

bitflags! {
    /// Pattern flags
    #[derive(Default)]
    pub struct Flags: u32 {
        /// Set case-insensitive matching.
        const CASELESS = ffi::CH_FLAG_CASELESS;
        /// Matching a `.` will not exclude newlines.
        const DOTALL = ffi::CH_FLAG_DOTALL;
        /// Set multi-line anchoring.
        const MULTILINE = ffi::CH_FLAG_MULTILINE;
        /// Set single-match only mode.
        const SINGLEMATCH = ffi::CH_FLAG_SINGLEMATCH;
        /// Enable UTF-8 mode for this expression.
        const UTF8 = ffi::CH_FLAG_UTF8;
        /// Enable Unicode property support for this expression.
        const UCP = ffi::CH_FLAG_UCP;
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
                '8' => flags |= Flags::UTF8,
                'W' => flags |= Flags::UCP,
                _ => {
                    return Err(Error::InvalidFlag(c));
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
        if self.contains(Flags::UTF8) {
            write!(f, "8")?
        }
        if self.contains(Flags::UCP) {
            write!(f, "W")?
        }
        Ok(())
    }
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
}

impl Pattern {
    /// Construct a pattern with expression.
    pub fn new<S: Into<String>>(expr: S) -> Pattern {
        Pattern {
            expression: expr.into(),
            flags: Flags::empty(),
            id: None,
        }
    }

    /// Construct a pattern with expression and flags.
    pub fn with_flags<S: Into<String>>(expr: S, flags: Flags) -> Pattern {
        Pattern {
            expression: expr.into(),
            flags,
            id: None,
        }
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
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = self.id {
            write!(f, "{}:", id)?;
        }

        if self.id.is_some() || !self.flags.is_empty() {
            write!(f, "/{}/{}", self.expression, self.flags)
        } else {
            write!(f, "{}", self.expression)
        }
    }
}

impl FromStr for Pattern {
    type Err = Error;

    fn from_str(expr: &str) -> Result<Self, Self::Err> {
        let (id, expr) = match expr.find(":/") {
            Some(off) => (Some(expr[..off].parse()?), &expr[off + 1..]),
            None => (None, expr),
        };
        let pattern = match (expr.starts_with('/'), expr.rfind('/')) {
            (true, Some(end)) if end > 0 => Pattern {
                expression: expr[1..end].into(),
                flags: expr[end + 1..].parse()?,
                id,
            },

            _ => Pattern {
                expression: expr.into(),
                flags: Flags::empty(),
                id,
            },
        };

        Ok(pattern)
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
            .collect::<Result<Vec<_>, Error>>()
            .map(Self)
    }
}
