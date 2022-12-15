use crate::ffi;

/// Compile mode
pub trait Mode {
    /// Id of mode
    const ID: u32;
    /// Name of mode
    const NAME: &'static str;

    /// The given database is a block database.
    fn is_block() -> bool {
        Self::ID == Block::ID
    }

    /// The given database is a block database.
    fn is_vectored() -> bool {
        Self::ID == Vectored::ID
    }

    /// The given database is a block database.
    fn is_streaming() -> bool {
        Self::ID == Streaming::ID
    }
}

/// Block scan (non-streaming) database.
#[derive(Debug, PartialEq, Eq)]
pub enum Block {}

/// Vectored scanning database.
#[derive(Debug, PartialEq, Eq)]
pub enum Vectored {}

/// Streaming database.
#[derive(Debug, PartialEq, Eq)]
pub enum Streaming {}

impl Mode for Block {
    const ID: u32 = ffi::HS_MODE_BLOCK;
    const NAME: &'static str = "Block";
}

impl Mode for Streaming {
    const ID: u32 = ffi::HS_MODE_STREAM;
    const NAME: &'static str = "Streaming";
}

impl Mode for Vectored {
    const ID: u32 = ffi::HS_MODE_VECTORED;
    const NAME: &'static str = "Vectored";
}
