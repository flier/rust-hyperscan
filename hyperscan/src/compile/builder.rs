use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use std::str::FromStr;

use foreign_types::{ForeignType, ForeignTypeRef};
use libc::c_char;

use crate::{
    common::{Database, Mode},
    compile::{AsCompileResult, Flags, Pattern, Patterns, PlatformRef},
    ffi, Error,
};

#[cfg(feature = "literal")]
use crate::compile::{Literal, LiteralFlags, Literals};

/// The regular expression pattern database builder.
pub trait Builder {
    /// The associated error which can be returned from compiling.
    type Err;

    /// Build an expression is compiled into a Hyperscan database which can be passed to the runtime functions
    fn build<T: Mode>(&self) -> Result<Database<T>, Self::Err> {
        self.for_platform(None)
    }

    /// Build an expression is compiled into a Hyperscan database for a target platform.
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>, Self::Err>;
}

/// Compile an expression into a Hyperscan database.
///
/// # Examples
///
/// ```rust
/// # use hyperscan::prelude::*;
/// let db: BlockDatabase = compile(r"/foo(bar)?/i").unwrap();
/// let mut s = db.alloc_scratch().unwrap();
///
/// let mut matches = vec![];
/// db.scan("hello foobar!", &mut s, |_, from, to, _| {
///     matches.push(from..to);
///     Matching::Continue
/// }).unwrap();
///
/// assert_eq!(matches, vec![0..9, 0..12]);
/// ```
pub fn compile<S: Builder, T: Mode>(expression: S) -> Result<Database<T>, S::Err> {
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
    /// # use hyperscan::prelude::*;
    /// let db: BlockDatabase = r"/foo(bar)?/i".build().unwrap();
    /// let mut s = db.alloc_scratch().unwrap();
    ///
    /// let mut matches = vec![];
    /// db.scan("hello foobar!", &mut s, |_, from, to, _| {
    ///     matches.push(from..to);
    ///     Matching::Continue
    /// }).unwrap();
    ///
    /// assert_eq!(matches, vec![0..9, 0..12]);
    /// ```
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>, Self::Err> {
        self.as_ref().parse::<Pattern>()?.for_platform(platform)
    }
}

impl Builder for Pattern {
    type Err = Error;

    ///
    /// The basic regular expression compiler.
    ///
    /// This is the function call with which an expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions
    ///
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>, Self::Err> {
        let expr = CString::new(self.expression.as_bytes())?;
        let mode = T::ID | if T::is_streaming() { self.som() } else { None }.map_or(0, |som| som as _);
        let mut db = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();

        unsafe {
            ffi::hs_compile(
                expr.as_bytes_with_nul().as_ptr() as *const c_char,
                self.flags.bits(),
                mode,
                platform.map_or_else(null_mut, ForeignTypeRef::as_ptr),
                db.as_mut_ptr(),
                err.as_mut_ptr(),
            )
            .ok_or_else(|| err.assume_init())
            .map(|_| Database::from_ptr(db.assume_init()))
            .map_err(|err| err.into())
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
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>, Self::Err> {
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
        let mode = T::ID | if T::is_streaming() { self.som() } else { None }.map_or(0, |som| som as _);
        let mut db = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();

        unsafe {
            ffi::hs_compile_multi(
                ptrs.as_ptr(),
                flags.as_ptr(),
                ids.as_ptr(),
                self.len() as u32,
                mode,
                platform.map_or_else(null_mut, ForeignTypeRef::as_ptr),
                db.as_mut_ptr(),
                err.as_mut_ptr(),
            )
            .ok_or_else(|| err.assume_init())
            .map(|_| Database::from_ptr(db.assume_init()))
            .map_err(|err| err.into())
        }
    }
}

#[cfg(feature = "literal")]
impl Builder for Literal {
    type Err = Error;

    ///
    /// The basic regular expression compiler.
    ///
    /// / This is the function call with which an expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions
    ///
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>, Self::Err> {
        let mode = T::ID | if T::is_streaming() { self.som() } else { None }.map_or(0, |som| som as _);
        let mut db = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();

        unsafe {
            ffi::hs_compile_lit(
                self.expression.as_ptr() as *const _,
                self.flags.bits(),
                self.expression.len(),
                mode,
                platform.map_or_else(null_mut, ForeignTypeRef::as_ptr),
                db.as_mut_ptr(),
                err.as_mut_ptr(),
            )
            .ok_or_else(|| err.assume_init())
            .map(|_| Database::from_ptr(db.assume_init()))
            .map_err(|err| err.into())
        }
    }
}

#[cfg(feature = "literal")]
impl Builder for Literals {
    type Err = Error;

    ///
    /// The multiple regular expression compiler.
    ///
    /// This is the function call with which a set of expressions is compiled into a database
    /// which can be passed to the runtime functions.
    /// Each expression can be labelled with a unique integer
    // which is passed into the match callback to identify the pattern that has matched.
    ///
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>, Self::Err> {
        let ptrs = self
            .iter()
            .map(|Literal { expression, .. }| expression.as_ptr() as *const _)
            .collect::<Vec<_>>();
        let lens = self
            .iter()
            .map(|Literal { expression, .. }| expression.len())
            .collect::<Vec<_>>();
        let flags = self
            .iter()
            .map(|Literal { flags, .. }| flags.bits() as _)
            .collect::<Vec<_>>();
        let ids = self
            .iter()
            .enumerate()
            .map(|(i, Literal { id, .. })| id.unwrap_or(i) as _)
            .collect::<Vec<_>>();
        let mode = T::ID | if T::is_streaming() { self.som() } else { None }.map_or(0, |som| som as _);
        let mut db = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();

        unsafe {
            ffi::hs_compile_lit_multi(
                ptrs.as_ptr(),
                flags.as_ptr(),
                ids.as_ptr(),
                lens.as_ptr(),
                self.len() as u32,
                mode,
                platform.map_or_else(null_mut, ForeignTypeRef::as_ptr),
                db.as_mut_ptr(),
                err.as_mut_ptr(),
            )
            .ok_or_else(|| err.assume_init())
            .map(|_| Database::from_ptr(db.assume_init()))
            .map_err(|err| err.into())
        }
    }
}

impl<T: Mode> Database<T> {
    /// The basic regular expression compiler.
    ///
    /// This is the function call with which an expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions.
    pub fn compile<S: Into<String>>(
        expression: S,
        flags: Flags,
        platform: Option<&PlatformRef>,
    ) -> Result<Database<T>, Error> {
        Pattern::with_flags(expression, flags)?.for_platform(platform)
    }

    /// The pure literal expression compiler.
    ///
    /// This is the function call with which an pure literal expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions.
    #[cfg(feature = "literal")]
    pub fn compile_literal<S: Into<String>>(
        expression: S,
        flags: LiteralFlags,
        platform: Option<&PlatformRef>,
    ) -> Result<Database<T>, Error> {
        Literal::with_flags(expression, flags)?.for_platform(platform)
    }
}

impl<T: Mode> FromStr for Database<T> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Pattern>()?.build::<T>()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::common::tests::validate_database;
    use crate::compile::{Flags, Platform};
    use crate::prelude::*;

    #[test]
    fn test_database_compile() {
        let info = Platform::host().unwrap();

        let db = BlockDatabase::compile("test", Flags::empty(), Some(&info)).unwrap();

        validate_database(&db);
    }
}
