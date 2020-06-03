use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

use anyhow::{bail, Error};
use bitflags::bitflags;
use derive_more::{Deref, DerefMut, From, Index, IndexMut, Into, IntoIterator};

use crate::chimera::ffi;

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
#[derive(Clone, Debug, PartialEq)]
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

    /// Parse a basic regular expression to a pattern.
    pub fn parse<S: AsRef<str>>(s: S) -> Result<Pattern, Error> {
        let expr = s.as_ref();
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

        debug!("pattern `{}` parsed to `{}`", expr, pattern);

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
            write!(f, "/{}/{}", expr, self.flags)
        } else {
            write!(f, "{}", expr)
        }
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
            .collect::<Result<Vec<_>, Error>>()
            .map(Self)
    }
}
