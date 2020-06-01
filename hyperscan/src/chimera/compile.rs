use std::ffi::CString;
use std::fmt;
use std::ptr::{null, null_mut};
use std::str::FromStr;

use anyhow::{bail, Error};
use bitflags::bitflags;
use foreign_types::{ForeignType, ForeignTypeRef};

use crate::chimera::{errors::AsCompileResult, ffi, Database};
use crate::{Pattern, Patterns, PlatformRef};

bitflags! {
    /// Pattern flags
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

/// Compile mode flags
///
/// The mode flags are used as values for the mode parameter of the various
/// compile calls `Builder::build` for `Pattern` or `Patterns`.
///
/// By default, the matcher will only supply the start and end offsets of the
/// match when the match callback is called. Using mode flag `Mode::Groups`
/// will also fill the `captured' array with the start and end offsets of all
/// the capturing groups specified by the pattern that has matched.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Mode {
    /// Disable capturing groups.
    NoGroups = ffi::CH_MODE_NOGROUPS,
    /// Enable capturing groups.
    Groups = ffi::CH_MODE_GROUPS,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::NoGroups
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

/// Define match limits for PCRE runtime.
pub struct MatchLimit {
    /// A limit from pcre_extra on the amount of match function called in PCRE to limit backtracking that can take place.
    pub max_matches: u64,
    /// A limit from pcre_extra on the recursion depth of match function in PCRE.
    pub recursion_depth: u64,
}

/// The regular expression pattern database builder.
pub trait Builder {
    /// The associated error which can be returned from compiling.
    type Err;

    /// Build an expression is compiled into a Hyperscan database which can be passed to the runtime functions
    fn build(&self) -> Result<Database, Self::Err> {
        self.for_platform(Mode::default(), None, None)
    }

    /// Build an expression is compiled into a Hyperscan database for a target platform.
    fn for_platform(
        &self,
        mode: Mode,
        match_limit: Option<MatchLimit>,
        platform: Option<&PlatformRef>,
    ) -> Result<Database, Self::Err>;
}

impl Builder for Pattern {
    type Err = Error;

    ///
    /// The basic regular expression compiler.
    ///
    /// This is the function call with which an expression is compiled into a Chimera database
    /// which can be passed to the runtime function.
    ///
    fn for_platform(
        &self,
        mode: Mode,
        match_limit: Option<MatchLimit>,
        platform: Option<&PlatformRef>,
    ) -> Result<Database, Self::Err> {
        let expr = CString::new(self.expression.as_str())?;
        let ptr = expr.as_bytes_with_nul().as_ptr() as *const i8;
        let flags = self.flags.bits();
        let mut db = null_mut();
        let mut err = null_mut();

        unsafe {
            if let Some(MatchLimit {
                max_matches,
                recursion_depth,
            }) = match_limit
            {
                let id = 0;

                ffi::ch_compile_ext_multi(
                    &ptr,
                    &flags,
                    &id,
                    1,
                    mode as _,
                    max_matches,
                    recursion_depth,
                    platform.map_or_else(null, |platform| platform.as_ptr() as *const _),
                    &mut db,
                    &mut err,
                )
            } else {
                ffi::ch_compile(
                    ptr,
                    flags,
                    mode as u32,
                    platform.map_or_else(null, |platform| platform.as_ptr() as *const _),
                    &mut db,
                    &mut err,
                )
            }
            .ok_or(err)
            .map(|_| Database::from_ptr(db))
        }
    }
}

impl Builder for Patterns {
    type Err = Error;

    ///
    /// The multiple regular expression compiler.
    ///
    /// This is the function call with which a set of expressions is compiled into a database
    /// which can be passed to the runtime functions.
    /// Each expression can be labelled with a unique integer
    // which is passed into the match callback to identify the pattern that has matched.
    ///
    fn for_platform(
        &self,
        mode: Mode,
        match_limit: Option<MatchLimit>,
        platform: Option<&PlatformRef>,
    ) -> Result<Database, Self::Err> {
        let expressions = self
            .iter()
            .map(|pattern| CString::new(pattern.expression.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let flags = self.iter().map(|pattern| pattern.flags.bits() as _).collect::<Vec<_>>();
        let ids = self
            .iter()
            .enumerate()
            .map(|(id, pattern)| pattern.id.unwrap_or(id) as _)
            .collect::<Vec<_>>();

        let ptrs = expressions
            .iter()
            .map(|expr| expr.as_ptr() as *const _)
            .collect::<Vec<_>>();

        let mut db = null_mut();
        let mut err = null_mut();

        unsafe {
            if let Some(MatchLimit {
                max_matches,
                recursion_depth,
            }) = match_limit
            {
                ffi::ch_compile_ext_multi(
                    ptrs.as_ptr(),
                    flags.as_ptr(),
                    ids.as_ptr(),
                    self.len() as _,
                    mode as _,
                    max_matches,
                    recursion_depth,
                    platform.map_or_else(null, |platform| platform.as_ptr() as *const _),
                    &mut db,
                    &mut err,
                )
            } else {
                ffi::ch_compile_multi(
                    ptrs.as_ptr(),
                    flags.as_ptr(),
                    ids.as_ptr(),
                    self.len() as _,
                    mode as _,
                    platform.map_or_else(null, |platform| platform.as_ptr() as *const _),
                    &mut db,
                    &mut err,
                )
            }
            .ok_or(err)
            .map(|_| Database::from_ptr(db))
        }
    }
}
