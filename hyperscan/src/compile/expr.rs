use std::ffi::CString;
use std::fmt::{self, Write};
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::str::FromStr;

use bitflags::bitflags;
use derive_more::{From, Into};
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};
use libc::c_char;
use thiserror::Error;

use crate::{
    compile::{AsCompileResult, Pattern},
    ffi, Result,
};

/// Hyperscan Error Codes
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("missing parameter")]
    MissingParameter,

    #[error("missing value")]
    MissingValue,

    #[error("unexpected parameter {0}")]
    UnexpectedParameter(String),
}

bitflags! {
    /// These flags are used in `hs_expr_ext_t::flags` to indicate which fields are used.
    #[derive(Default)]
    struct Flags: u64 {
        const MIN_OFFSET = ffi::HS_EXT_FLAG_MIN_OFFSET as u64;
        const MAX_OFFSET = ffi::HS_EXT_FLAG_MAX_OFFSET as u64;
        const MIN_LENGTH = ffi::HS_EXT_FLAG_MIN_LENGTH as u64;
        const EDIT_DISTANCE = ffi::HS_EXT_FLAG_EDIT_DISTANCE as u64;
        const HAMMING_DISTANCE = ffi::HS_EXT_FLAG_HAMMING_DISTANCE as u64;
    }
}

/// A structure containing additional parameters related to an expression.
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, From, Into)]
pub struct ExprExt(ffi::hs_expr_ext_t);

impl fmt::Debug for ExprExt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("ExprExt");
        let flags = self.flags();
        if flags.contains(Flags::MIN_OFFSET) {
            s.field("min_offset", &self.0.min_offset);
        }
        if flags.contains(Flags::MAX_OFFSET) {
            s.field("max_offset", &self.0.max_offset);
        }
        if flags.contains(Flags::MIN_LENGTH) {
            s.field("min_length", &self.0.min_length);
        }
        if flags.contains(Flags::EDIT_DISTANCE) {
            s.field("edit_distance", &self.0.edit_distance);
        }
        if flags.contains(Flags::HAMMING_DISTANCE) {
            s.field("hamming_distance", &self.0.hamming_distance);
        }
        s.finish()
    }
}

impl fmt::Display for ExprExt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let flags = self.flags();
        let mut fields = vec![];

        if flags.contains(Flags::MIN_OFFSET) {
            fields.push(format!("min_offset={}", self.0.min_offset));
        }
        if flags.contains(Flags::MAX_OFFSET) {
            fields.push(format!("max_offset={}", self.0.max_offset));
        }
        if flags.contains(Flags::MIN_LENGTH) {
            fields.push(format!("min_length={}", self.0.min_length));
        }
        if flags.contains(Flags::EDIT_DISTANCE) {
            fields.push(format!("edit_distance={}", self.0.edit_distance));
        }
        if flags.contains(Flags::HAMMING_DISTANCE) {
            fields.push(format!("hamming_distance={}", self.0.hamming_distance));
        }

        f.write_char('{')?;
        f.write_str(&fields.join(","))?;
        f.write_char('}')
    }
}

impl FromStr for ExprExt {
    type Err = crate::Error;

    fn from_str(mut s: &str) -> Result<Self> {
        if s.starts_with('{') && s.ends_with('}') {
            s = &s[1..s.len() - 1];
        }

        s.split(',')
            .map(|kv| kv.splitn(2, '='))
            .fold(Ok(ExprExt::default()), |ext, mut kv| {
                ext.and_then(|mut ext| {
                    let key = kv.next().ok_or(Error::MissingParameter)?;
                    let value = kv.next().ok_or(Error::MissingValue)?;

                    match key {
                        "min_offset" => {
                            ext.set_min_offset(value.parse()?);
                        }
                        "max_offset" => {
                            ext.set_max_offset(value.parse()?);
                        }
                        "min_length" => {
                            ext.set_min_length(value.parse()?);
                        }
                        "edit_distance" => {
                            ext.set_edit_distance(value.parse()?);
                        }
                        "hamming_distance" => {
                            ext.set_hamming_distance(value.parse()?);
                        }
                        _ => return Err(Error::UnexpectedParameter(key.into()).into()),
                    }

                    Ok(ext)
                })
            })
    }
}

impl ExprExt {
    fn flags(&self) -> Flags {
        Flags::from_bits_truncate(self.0.flags)
    }

    fn set_flags(&mut self, flags: Flags) {
        self.0.flags |= flags.bits();
    }

    /// Returns true if the expression contains no additional parameters.
    pub fn is_empty(&self) -> bool {
        self.flags().is_empty()
    }

    /// The minimum end offset in the data stream at which this expression should match successfully.
    pub fn min_offset(&self) -> Option<u64> {
        if self.flags().contains(Flags::MIN_OFFSET) {
            Some(self.0.min_offset)
        } else {
            None
        }
    }

    /// The maximum end offset in the data stream at which this expression should match successfully.
    pub fn max_offset(&self) -> Option<u64> {
        if self.flags().contains(Flags::MAX_OFFSET) {
            Some(self.0.max_offset)
        } else {
            None
        }
    }

    /// The minimum match length (from start to end) required to successfully match this expression.
    pub fn min_length(&self) -> Option<u64> {
        if self.flags().contains(Flags::MIN_LENGTH) {
            Some(self.0.min_length)
        } else {
            None
        }
    }

    /// Allow patterns to approximately match within this edit distance.
    pub fn edit_distance(&self) -> Option<u32> {
        if self.flags().contains(Flags::EDIT_DISTANCE) {
            Some(self.0.edit_distance)
        } else {
            None
        }
    }

    /// Allow patterns to approximately match within this Hamming distance.
    pub fn hamming_distance(&self) -> Option<u32> {
        if self.flags().contains(Flags::HAMMING_DISTANCE) {
            Some(self.0.hamming_distance)
        } else {
            None
        }
    }

    /// Sets the value for the minimum end offset in the data stream at which this expression should match successfully.
    pub fn set_min_offset(&mut self, min_offset: u64) -> &mut Self {
        self.set_flags(Flags::MIN_OFFSET);
        self.0.min_offset = min_offset;
        self
    }

    /// Sets the value for the maximum end offset in the data stream at which this expression should match successfully.
    pub fn set_max_offset(&mut self, max_offset: u64) -> &mut Self {
        self.set_flags(Flags::MAX_OFFSET);
        self.0.max_offset = max_offset;
        self
    }

    /// Sets the value for the minimum match length (from start to end) required to successfully match this expression.
    pub fn set_min_length(&mut self, min_length: u64) -> &mut Self {
        self.set_flags(Flags::MIN_LENGTH);
        self.0.min_length = min_length;
        self
    }

    /// Sets the value that allow patterns to approximately match within this edit distance.
    pub fn set_edit_distance(&mut self, edit_distance: u32) -> &mut Self {
        self.set_flags(Flags::EDIT_DISTANCE);
        self.0.edit_distance = edit_distance;
        self
    }

    /// Sets the value that allow patterns to approximately match within this Hamming distance.
    pub fn set_hamming_distance(&mut self, hamming_distance: u32) -> &mut Self {
        self.set_flags(Flags::HAMMING_DISTANCE);
        self.0.hamming_distance = hamming_distance;
        self
    }
}

foreign_type! {
    /// A type containing information related to an expression
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hyperscan::prelude::*;
    /// let pattern: Pattern = r"(\d{4})-(\d{2})-(\d{2,4})".parse().unwrap();
    /// let info = pattern.info().unwrap();
    ///
    /// assert_eq!(info.min_width(), 10);
    /// assert_eq!(info.max_width(), 12);
    /// assert!(!info.unordered_matches());
    /// assert!(!info.matches_at_eod());
    /// assert!(!info.matches_only_at_eod());
    /// ```
    pub unsafe type ExprInfo: Send + Sync {
        type CType = ffi::hs_expr_info;

        fn drop = drop_expr_info;
    }
}

unsafe fn drop_expr_info(info: *mut ffi::hs_expr_info) {
    libc::free(info as *mut _);
}

impl Deref for ExprInfoRef {
    type Target = ffi::hs_expr_info;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_ptr() }
    }
}

impl fmt::Debug for ExprInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExprInfo")
            .field("min_width", &self.min_width)
            .field("max_width", &self.max_width)
            .field("unordered_matches", &self.unordered_matches())
            .field("matches_at_eod", &self.matches_at_eod())
            .field("matches_only_at_eod", &self.matches_only_at_eod())
            .finish()
    }
}

impl ExprInfoRef {
    /// The minimum length in bytes of a match for the pattern.
    pub fn min_width(&self) -> usize {
        self.min_width as usize
    }

    /// The maximum length in bytes of a match for the pattern.
    pub fn max_width(&self) -> usize {
        self.max_width as usize
    }

    /// Whether this expression can produce matches that are not returned in order,
    /// such as those produced by assertions.
    pub fn unordered_matches(&self) -> bool {
        self.unordered_matches != 0
    }

    /// Whether this expression can produce matches at end of data (EOD).
    pub fn matches_at_eod(&self) -> bool {
        self.matches_at_eod != 0
    }

    /// Whether this expression can *only* produce matches at end of data (EOD).
    pub fn matches_only_at_eod(&self) -> bool {
        self.matches_only_at_eod != 0
    }
}

impl Pattern {
    ///
    /// Utility function providing information about a regular expression.
    ///
    /// The information provided in ExpressionInfo
    /// includes the minimum and maximum width of a pattern match.
    ///
    pub fn info(&self) -> Result<ExprInfo> {
        let expr = CString::new(self.expression.as_str())?;
        let mut info = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();

        let info = unsafe {
            ffi::hs_expression_ext_info(
                expr.as_ptr() as *const c_char,
                self.flags.bits(),
                &self.ext.0 as *const _,
                info.as_mut_ptr(),
                err.as_mut_ptr(),
            )
            .ok_or_else(|| err.assume_init())?;

            ExprInfo::from_ptr(info.assume_init())
        };

        Ok(info)
    }
}
