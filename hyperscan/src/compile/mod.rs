mod error;
#[macro_use]
mod pattern;
mod builder;
mod expr;
#[macro_use]
#[cfg(feature = "literal")]
mod literal;
mod platform;

pub use self::builder::{compile, Builder};
pub use self::error::{AsCompileResult, Error};
#[doc(hidden)]
#[deprecated = "use `ExprExt` instead"]
pub use self::expr::ExprExt as ExpressionExt;
#[doc(hidden)]
#[deprecated = "use `ExprInfo` instead"]
pub use self::expr::ExprInfo as ExpressionInfo;
pub use self::expr::{Error as ExprError, ExprExt, ExprInfo};
#[cfg(feature = "literal")]
pub use self::literal::{Flags as LiteralFlags, Literal, Literals};
pub use self::pattern::{Flags, Pattern, Patterns, SomHorizon};
pub use self::platform::{CpuFeatures, Platform, PlatformRef, Tune};
