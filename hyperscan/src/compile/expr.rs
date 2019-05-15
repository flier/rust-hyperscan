use core::ptr::null_mut;
use std::ffi::CString;

use failure::Error;
use foreign_types::ForeignType;

use crate::compile::Pattern;

/// A type containing information related to an expression
#[derive(Debug, Copy, Clone)]
pub struct ExpressionInfo {
    /// The minimum length in bytes of a match for the pattern.
    pub min_width: usize,

    /// The maximum length in bytes of a match for the pattern.
    pub max_width: usize,

    /// Whether this expression can produce matches that are not returned in order,
    /// such as those produced by assertions.
    pub unordered_matches: bool,

    /// Whether this expression can produce matches at end of data (EOD).
    pub matches_at_eod: bool,

    /// Whether this expression can *only* produce matches at end of data (EOD).
    pub matches_only_at_eod: bool,
}

impl Pattern {
    ///
    /// Utility function providing information about a regular expression.
    ///
    /// The information provided in ExpressionInfo
    /// includes the minimum and maximum width of a pattern match.
    ///
    pub fn info(&self) -> Result<ExpressionInfo, Error> {
        let expr = CString::new(self.expression.as_str())?;
        let mut info = null_mut();
        let mut err = null_mut();

        unsafe {
            check_compile_error!(
                ffi::hs_expression_info(expr.as_ptr() as *const i8, self.flags.bits(), &mut info, &mut err),
                err
            );

            let info = info.as_ref().unwrap();
            let info = ExpressionInfo {
                min_width: info.min_width as usize,
                max_width: info.max_width as usize,
                unordered_matches: info.unordered_matches != 0,
                matches_at_eod: info.matches_at_eod != 0,
                matches_only_at_eod: info.matches_only_at_eod != 0,
            };

            debug!("expression `{}` info: {:?}", self, info);

            Ok(info)
        }
    }
}
