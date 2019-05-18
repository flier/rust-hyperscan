mod error;
#[macro_use]
mod pattern;
mod builder;
mod expr;
mod platform;

pub use self::builder::Builder;
pub use self::error::{AsCompileResult, Error};
pub use self::expr::Info as ExpressionInfo;
pub use self::pattern::{Ext as ExpressionExt, Flags, Pattern, Patterns, SomHorizon};
pub use self::platform::{CpuFeatures, Platform, PlatformRef, Tune};
