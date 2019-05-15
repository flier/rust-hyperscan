use core::fmt;
use core::str::FromStr;

use failure::{bail, Error};

use crate::constants::*;

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
            write!(f, "i")?
        }
        if self.is_set(HS_FLAG_MULTILINE) {
            write!(f, "m")?
        }
        if self.is_set(HS_FLAG_DOTALL) {
            write!(f, "s")?
        }
        if self.is_set(HS_FLAG_SINGLEMATCH) {
            write!(f, "H")?
        }
        if self.is_set(HS_FLAG_ALLOWEMPTY) {
            write!(f, "V")?
        }
        if self.is_set(HS_FLAG_UTF8) {
            write!(f, "8")?
        }
        if self.is_set(HS_FLAG_UCP) {
            write!(f, "W")?
        }
        if self.is_set(HS_FLAG_COMBINATION) {
            write!(f, "C")?
        }
        if self.is_set(HS_FLAG_QUIET) {
            write!(f, "Q")?
        }
        Ok(())
    }
}

impl CompileFlags {
    #[inline]
    pub fn is_set(&self, flag: u32) -> bool {
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
                _ => {
                    bail!("invalid compile flag: {}", c);
                }
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
        unsafe {
            let (id, expr) = match s.find(':') {
                Some(off) => (s.get_unchecked(0..off).parse()?, s.get_unchecked(off + 1..s.len())),
                None => (0, s),
            };

            let pattern = match (expr.starts_with('/'), expr.rfind('/')) {
                (true, Some(end)) if end > 0 => Pattern {
                    expression: String::from(expr.get_unchecked(1..end)),
                    flags: CompileFlags::parse(expr.get_unchecked(end + 1..expr.len()))?,
                    id: id,
                },

                _ => Pattern {
                    expression: String::from(expr),
                    flags: CompileFlags::default(),
                    id: id,
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
        pattern!($expr, flags => 0, id => 0)
    }};
    ($expr:expr, flags => $flags:expr) => {{
        pattern!($expr, flags => $flags, id => 0)
    }};
    ($expr:expr, flags => $flags:expr, id => $id:expr) => {{
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
            v.push(pattern!{$expr, flags => $flags, id => v.len() + 1});
        )*

        v
    }};
}
