use std::fmt;
use std::result::Result as StdResult;

use thiserror::Error;

use crate::{common::Error as HsError, ffi};

/// The type returned by hyperscan methods.
pub type Result<T> = StdResult<T, Error>;

/// Hyperscan Error
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// Hyperscan error
    #[error(transparent)]
    Hyperscan(#[from] crate::common::Error),

    /// Chimera error
    #[cfg(feature = "chimera")]
    #[error(transparent)]
    Chimera(#[from] crate::chimera::Error),

    /// Expression error
    #[error(transparent)]
    Expr(#[from] crate::compile::ExprError),

    /// Invalid UTF-8 string
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    /// Parse integer error
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// Parse C string error
    #[error(transparent)]
    NulByte(#[from] std::ffi::NulError),

    /// Invalid flag
    #[error("invalid pattern flag: {0}")]
    InvalidFlag(char),
}

pub trait AsResult
where
    Self: Sized,
{
    type Output;
    type Error: fmt::Debug;

    fn ok(self) -> StdResult<Self::Output, Self::Error>;

    fn map<U, F: FnOnce(Self::Output) -> U>(self, op: F) -> StdResult<U, Self::Error> {
        self.ok().map(op)
    }

    fn and_then<U, F: FnOnce(Self::Output) -> StdResult<U, Self::Error>>(self, op: F) -> StdResult<U, Self::Error> {
        self.ok().and_then(op)
    }

    fn expect(self, msg: &str) -> Self::Output {
        self.ok().expect(msg)
    }
}

impl AsResult for ffi::hs_error_t {
    type Output = ();
    type Error = Error;

    fn ok(self) -> StdResult<Self::Output, Self::Error> {
        if self == ffi::HS_SUCCESS as ffi::hs_error_t {
            Ok(())
        } else {
            Err(HsError::from(self).into())
        }
    }
}
