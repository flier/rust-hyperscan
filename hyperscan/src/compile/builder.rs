use std::ffi::CString;
use std::ptr::null_mut;

use anyhow::Result;
use foreign_types::{ForeignType, ForeignTypeRef};
use libc::c_uint;

use crate::common::{Database, Mode};
use crate::compile::{
    AsCompileResult, Flags, Literal, LiteralFlags, Literals, Pattern, Patterns, PlatformRef, SomHorizon,
};
use crate::ffi;

/// The regular expression pattern database builder.
pub trait Builder {
    /// Build an expression is compiled into a Hyperscan database which can be passed to the runtime functions
    fn build<T: Mode>(&self) -> Result<Database<T>> {
        self.for_platform(None)
    }

    /// Build an expression is compiled into a Hyperscan database for a target platform.
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>>;
}

impl Builder for Pattern {
    ///
    /// The basic regular expression compiler.
    ///
    /// / This is the function call with which an expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions
    ///
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>> {
        let expr = CString::new(self.expression.as_bytes())?;
        let mut mode = T::ID;
        let mut db = null_mut();
        let mut err = null_mut();

        if T::ID == ffi::HS_MODE_STREAM && self.flags.contains(Flags::SOM_LEFTMOST) {
            mode |= self.som.unwrap_or(SomHorizon::Medium) as u32;
        }

        unsafe {
            ffi::hs_compile(
                expr.as_bytes_with_nul().as_ptr() as *const i8,
                self.flags.bits(),
                mode,
                platform.map_or_else(null_mut, ForeignTypeRef::as_ptr),
                &mut db,
                &mut err,
            )
            .ok_or(err)
            .map(|_| Database::from_ptr(db))
        }
    }
}

impl Builder for Patterns {
    ///
    /// The multiple regular expression compiler.
    ///
    /// This is the function call with which a set of expressions is compiled into a database
    /// which can be passed to the runtime functions.
    /// Each expression can be labelled with a unique integer
    // which is passed into the match callback to identify the pattern that has matched.
    ///
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>> {
        let mut expressions = Vec::with_capacity(self.len());
        let mut ptrs = Vec::with_capacity(self.len());
        let mut flags = Vec::with_capacity(self.len());
        let mut ids = Vec::with_capacity(self.len());
        let mut mode = T::ID;

        if T::ID == ffi::HS_MODE_STREAM && self.iter().any(|pattern| pattern.flags.contains(Flags::SOM_LEFTMOST)) {
            mode |= self
                .iter()
                .flat_map(|pattern| pattern.som)
                .max()
                .unwrap_or(SomHorizon::Medium) as u32;
        }

        for (i, pattern) in self.iter().enumerate() {
            let expr = CString::new(pattern.expression.as_str())?;

            expressions.push(expr);
            flags.push(pattern.flags.bits() as c_uint);
            ids.push(pattern.id.unwrap_or(i) as u32);
        }

        for expr in &expressions {
            ptrs.push(expr.as_ptr() as *const i8);
        }

        let mut db = null_mut();
        let mut err = null_mut();

        unsafe {
            ffi::hs_compile_multi(
                ptrs.as_ptr(),
                flags.as_ptr(),
                ids.as_ptr(),
                self.len() as u32,
                mode,
                platform.map_or_else(null_mut, ForeignTypeRef::as_ptr),
                &mut db,
                &mut err,
            )
            .ok_or(err)
            .map(|_| Database::from_ptr(db))
        }
    }
}

impl Builder for Literal {
    ///
    /// The basic regular expression compiler.
    ///
    /// / This is the function call with which an expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions
    ///
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>> {
        let mut mode = T::ID;
        let mut db = null_mut();
        let mut err = null_mut();

        if T::ID == ffi::HS_MODE_STREAM && self.flags.contains(LiteralFlags::SOM_LEFTMOST) {
            mode |= self.som.unwrap_or(SomHorizon::Medium) as u32;
        }

        unsafe {
            ffi::hs_compile_lit(
                self.expression.as_ptr() as *const _,
                self.flags.bits(),
                self.expression.len(),
                mode,
                platform.map_or_else(null_mut, ForeignTypeRef::as_ptr),
                &mut db,
                &mut err,
            )
            .ok_or(err)
            .map(|_| Database::from_ptr(db))
        }
    }
}

impl Builder for Literals {
    ///
    /// The multiple regular expression compiler.
    ///
    /// This is the function call with which a set of expressions is compiled into a database
    /// which can be passed to the runtime functions.
    /// Each expression can be labelled with a unique integer
    // which is passed into the match callback to identify the pattern that has matched.
    ///
    fn for_platform<T: Mode>(&self, platform: Option<&PlatformRef>) -> Result<Database<T>> {
        let mut ptrs = Vec::with_capacity(self.len());
        let mut lens = Vec::with_capacity(self.len());
        let mut flags = Vec::with_capacity(self.len());
        let mut ids = Vec::with_capacity(self.len());
        let mut mode = T::ID;

        if T::ID == ffi::HS_MODE_STREAM
            && self
                .iter()
                .any(|pattern| pattern.flags.contains(LiteralFlags::SOM_LEFTMOST))
        {
            mode |= self
                .iter()
                .flat_map(|pattern| pattern.som)
                .max()
                .unwrap_or(SomHorizon::Medium) as u32;
        }

        for (i, pattern) in self.iter().enumerate() {
            ptrs.push(pattern.expression.as_ptr() as *const _);
            lens.push(pattern.expression.len());
            flags.push(pattern.flags.bits() as c_uint);
            ids.push(pattern.id.unwrap_or(i) as u32);
        }

        let mut db = null_mut();
        let mut err = null_mut();

        unsafe {
            ffi::hs_compile_lit_multi(
                ptrs.as_ptr(),
                flags.as_ptr(),
                ids.as_ptr(),
                lens.as_ptr(),
                self.len() as u32,
                mode,
                platform.map_or_else(null_mut, ForeignTypeRef::as_ptr),
                &mut db,
                &mut err,
            )
            .ok_or(err)
            .map(|_| Database::from_ptr(db))
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
    ) -> Result<Database<T>> {
        Pattern::with_flags(expression, flags)?.for_platform(platform)
    }

    /// The pure literal expression compiler.
    ///
    /// This is the function call with which an pure literal expression is compiled
    /// into a Hyperscan database which can be passed to the runtime functions.
    pub fn compile_literal<S: Into<String>>(
        expression: S,
        flags: LiteralFlags,
        platform: Option<&PlatformRef>,
    ) -> Result<Database<T>> {
        Literal::with_flags(expression, flags)?.for_platform(platform)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::common::tests::validate_database;
    use crate::compile::{Flags, Platform};
    use crate::prelude::*;

    #[test]
    fn test_database_compile() {
        let _ = pretty_env_logger::try_init();
        let info = Platform::host().unwrap();

        let db = BlockDatabase::compile("test", Flags::empty(), Some(&info)).unwrap();

        validate_database(&db);
    }
}
