use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr::null;

use anyhow::Error;
use foreign_types::{ForeignType, ForeignTypeRef};

use crate::chimera::{errors::AsCompileResult, ffi, Database, Pattern, Patterns};
use crate::PlatformRef;

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

    /// Build an expression is compiled into a Chimera database which can be passed to the runtime functions.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hyperscan::chimera::prelude::*;
    /// let pattern: Pattern = "/test/i".parse().unwrap();
    /// let db = pattern.build().unwrap();
    /// let scratch = db.alloc_scratch().unwrap();
    /// let mut matches = vec![];
    /// let mut errors = vec![];
    ///
    /// db.scan("some test data", &scratch, |id, from, to, _flags, captured| {
    ///     println!("found pattern {} : {} @ [{}, {})", id, pattern.expression, from, to);
    ///
    ///     matches.push((from, to));
    ///
    ///     Matching::Continue
    /// }, |error_type, id| {
    ///     errors.push((error_type, id));
    ///
    ///     Matching::Skip
    /// }).unwrap();
    ///
    /// assert_eq!(matches, vec![(5, 9)]);
    /// assert_eq!(errors, vec![]);
    /// ```
    fn build(&self) -> Result<Database, Self::Err> {
        self.for_platform(Mode::NoGroups, None, None)
    }

    /// Build an expression is compiled into a Chimera database that the database as a whole for capturing groups.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use hyperscan::chimera::prelude::*;
    /// let pattern: Pattern = r"/(?<word>\w+)/i".parse().unwrap();
    /// let db = pattern.with_groups().unwrap();
    /// let scratch = db.alloc_scratch().unwrap();
    /// let mut matches = vec![];
    /// let mut captures = vec![];
    /// let mut errors = vec![];
    ///
    /// db.scan("some test data", &scratch, |id, from, to, _flags, captured| {
    ///     println!("found pattern {} : {} @ [{}, {}), captured {:?}", id, pattern.expression, from, to, captured);
    ///
    ///     matches.push((from, to));
    ///
    ///     if let Some(captured) = captured {
    ///         captures.push(captured.first().expect("captured").range());
    ///     }
    ///
    ///     Matching::Continue
    /// }, |error_type, id| {
    ///     errors.push((error_type, id));
    ///
    ///     Matching::Skip
    /// }).unwrap();
    ///
    /// assert_eq!(matches, vec![(0, 4), (5, 9), (10, 14)]);
    /// assert_eq!(captures, vec![0..4, 5..9, 10..14]);
    /// assert_eq!(errors, vec![]);
    /// ```
    fn with_groups(&self) -> Result<Database, Self::Err> {
        self.for_platform(Mode::Groups, None, None)
    }

    /// Build an expression is compiled into a Chimera database for a target platform.
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
        let mut db = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();

        unsafe {
            if let Some(MatchLimit {
                max_matches,
                recursion_depth,
            }) = match_limit
            {
                ffi::ch_compile_ext_multi(
                    &ptr,
                    &flags,
                    &0,
                    1,
                    mode as _,
                    max_matches,
                    recursion_depth,
                    platform.map_or_else(null, |platform| platform.as_ptr() as *const _),
                    db.as_mut_ptr(),
                    err.as_mut_ptr(),
                )
            } else {
                ffi::ch_compile(
                    ptr,
                    flags,
                    mode as u32,
                    platform.map_or_else(null, |platform| platform.as_ptr() as *const _),
                    db.as_mut_ptr(),
                    err.as_mut_ptr(),
                )
            }
            .ok_or_else(|| err.assume_init())
            .map(|_| Database::from_ptr(db.assume_init()))
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
        let mut db = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();

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
                    db.as_mut_ptr(),
                    err.as_mut_ptr(),
                )
            } else {
                ffi::ch_compile_multi(
                    ptrs.as_ptr(),
                    flags.as_ptr(),
                    ids.as_ptr(),
                    self.len() as _,
                    mode as _,
                    platform.map_or_else(null, |platform| platform.as_ptr() as *const _),
                    db.as_mut_ptr(),
                    err.as_mut_ptr(),
                )
            }
            .ok_or_else(|| err.assume_init())
            .map(|_| Database::from_ptr(db.assume_init()))
        }
    }
}
