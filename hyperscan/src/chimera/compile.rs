use std::ffi::CStr;
use std::ffi::CString;
use std::fmt;
use std::mem::MaybeUninit;
use std::ptr::null;
use std::str::FromStr;

use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};
use libc::c_char;

use crate::{
    chimera::{ffi, Database, Error as ChError, Pattern, Patterns},
    error::AsResult,
    Error, PlatformRef,
};

foreign_type! {
    /// Providing details of the compile error condition.
    pub unsafe type CompileError: Send + Sync {
        type CType = ffi::ch_compile_error_t;

        fn drop = free_compile_error;
    }
}

unsafe fn free_compile_error(err: *mut ffi::ch_compile_error_t) {
    ffi::ch_free_compile_error(err).expect("free compile error");
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl fmt::Debug for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompileError")
            .field("message", &self.message())
            .field("expression", &self.expression())
            .finish()
    }
}

impl PartialEq for CompileError {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl Eq for CompileError {}

impl CompileError {
    unsafe fn as_ref(&self) -> &ffi::ch_compile_error_t {
        self.as_ptr().as_ref().unwrap()
    }

    /// A human-readable error message describing the error.
    pub fn message(&self) -> &str {
        unsafe { CStr::from_ptr(self.as_ref().message).to_str().unwrap() }
    }

    /// The zero-based number of the expression that caused the error (if this can be determined).
    pub fn expression(&self) -> Option<usize> {
        let n = unsafe { self.as_ref().expression };

        if n < 0 {
            None
        } else {
            Some(n as usize)
        }
    }
}

pub trait AsCompileResult: Sized {
    type Output;
    type Err: fmt::Display;

    fn ok_or(self, err: *mut ffi::ch_compile_error_t) -> Result<Self::Output, Self::Err> {
        self.ok_or_else(|| err)
    }

    fn ok_or_else<F>(self, err: F) -> Result<Self::Output, Self::Err>
    where
        F: FnOnce() -> *mut ffi::ch_compile_error_t;
}

impl AsCompileResult for ffi::ch_error_t {
    type Output = ();
    type Err = Error;

    fn ok_or_else<F>(self, err: F) -> Result<Self::Output, Self::Err>
    where
        F: FnOnce() -> *mut ffi::ch_compile_error_t,
    {
        if self == ffi::CH_SUCCESS as ffi::ch_error_t {
            Ok(())
        } else if self == ffi::CH_COMPILER_ERROR {
            Err(ChError::CompileError(unsafe { CompileError::from_ptr(err()) }).into())
        } else {
            Err(ChError::from(self).into())
        }
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
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

/// Compile an expression into a Chimera database.
///
/// # Examples
///
/// ```rust
/// # use hyperscan::chimera::prelude::*;
/// let db: Database = compile(r"/foo(bar)?/i").unwrap();
/// let mut s = db.alloc_scratch().unwrap();
///
/// let mut matches = vec![];
/// db.scan("hello foobar!", &mut s, |_, from, to, _, _| {
///     matches.push(from..to);
///     Matching::Continue
/// }, |_, _|{
///     Matching::Skip
/// }).unwrap();
///
/// assert_eq!(matches, vec![6..12]);
/// ```
pub fn compile<S: Builder>(expression: S) -> Result<Database, S::Err> {
    expression.build()
}

impl<S> Builder for S
where
    S: AsRef<str>,
{
    type Err = Error;

    /// Build an expression is compiled into a Hyperscan database for a target platform.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hyperscan::chimera::prelude::*;
    /// let db: Database = r"/foo(bar)?/i".build().unwrap();
    /// let mut s = db.alloc_scratch().unwrap();
    ///
    /// let mut matches = vec![];
    /// db.scan("hello foobar!", &mut s, |_, from, to, _, _| {
    ///     matches.push(from..to);
    ///     Matching::Continue
    /// }, Matching::Skip).unwrap();
    ///
    /// assert_eq!(matches, vec![6..12]);
    /// ```
    fn for_platform(
        &self,
        mode: Mode,
        match_limit: Option<MatchLimit>,
        platform: Option<&PlatformRef>,
    ) -> Result<Database, Self::Err> {
        self.as_ref()
            .parse::<Pattern>()?
            .for_platform(mode, match_limit, platform)
    }
}

/// The regular expression pattern database builder.
pub trait Builder {
    /// The associated error which can be returned from compiling.
    type Err;

    /// Build an expression is compiled into a Chimera database which can be passed to the runtime functions.
    ///
    /// # Examples
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
    /// # Examples
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
    /// db.scan("some test data", &scratch, |id, from, to, _flags, captured: Option<&[Capture]>| {
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
        let ptr = expr.as_bytes_with_nul().as_ptr() as *const c_char;
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
            .map(|Pattern { expression, .. }| CString::new(expression.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let ptrs = expressions
            .iter()
            .map(|expr| expr.as_ptr() as *const _)
            .collect::<Vec<_>>();
        let flags = self
            .iter()
            .map(|Pattern { flags, .. }| flags.bits() as _)
            .collect::<Vec<_>>();
        let ids = self
            .iter()
            .enumerate()
            .map(|(i, Pattern { id, .. })| id.unwrap_or(i) as _)
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

impl FromStr for Database {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Pattern>()?.build()
    }
}
