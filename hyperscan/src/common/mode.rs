use crate::constants::*;

/// Compile mode
pub trait Mode {
    const ID: u32;
    const NAME: &'static str;
}

/// Block scan (non-streaming) database.
#[derive(Debug)]
pub enum Block {}

/// Streaming database.
#[derive(Debug)]
pub enum Streaming {}

/// Vectored scanning database.
#[derive(Debug)]
pub enum Vectored {}

impl Mode for Block {
    const ID: u32 = HS_MODE_BLOCK;
    const NAME: &'static str = "Block";
}

impl Mode for Streaming {
    const ID: u32 = HS_MODE_STREAM;
    const NAME: &'static str = "Streaming";
}

impl Mode for Vectored {
    const ID: u32 = HS_MODE_VECTORED;
    const NAME: &'static str = "Vectored";
}
