use std::ffi::CString;
use std::fmt;
use std::mem::MaybeUninit;
use std::ops::Deref;

use anyhow::Result;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::compile::{AsCompileResult, Pattern};
use crate::ffi;

foreign_type! {
    /// A type containing information related to an expression
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
        let ext = self.ext.into();
        let mut info = MaybeUninit::uninit();
        let mut err = MaybeUninit::uninit();

        let info = unsafe {
            ffi::hs_expression_ext_info(
                expr.as_ptr() as *const i8,
                self.flags.bits(),
                &ext,
                info.as_mut_ptr(),
                err.as_mut_ptr(),
            )
            .ok_or_else(|| err.assume_init())?;

            ExprInfo::from_ptr(info.assume_init())
        };

        debug!("expression `{}` info: {:?}", self, info);

        Ok(info)
    }
}
