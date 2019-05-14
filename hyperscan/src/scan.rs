use core::mem;

use failure::Error;
use foreign_types::{ForeignType, ForeignTypeRef};
use libc::c_uint;

use crate::api::{Block, MatchEventCallback, Vectored};
use crate::database::DatabaseRef;
use crate::errors::AsResult;
use crate::runtime::ScratchRef;
use crate::stream::Stream;

impl DatabaseRef<Block> {
    /// pattern matching takes place for block-mode pattern databases.
    pub fn scan<T, D>(
        &self,
        data: T,
        scratch: &ScratchRef,
        callback: MatchEventCallback<D>,
        context: Option<&D>,
    ) -> Result<(), Error>
    where
        T: AsRef<[u8]>,
    {
        unsafe {
            let bytes = data.as_ref();

            ffi::hs_scan(
                self.as_ptr(),
                bytes.as_ptr() as *const i8,
                bytes.len() as u32,
                0,
                scratch.as_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            )
            .ok()
            .map(|_| ())
        }
    }
}

impl DatabaseRef<Vectored> {
    /// pattern matching takes place for vectoring-mode pattern databases.
    pub fn scan<D>(
        &self,
        data: &[&[u8]],
        scratch: &ScratchRef,
        callback: MatchEventCallback<D>,
        context: Option<&D>,
    ) -> Result<(), Error> {
        let mut ptrs = Vec::with_capacity(data.len());
        let mut lens = Vec::with_capacity(data.len());

        for v in data {
            ptrs.push(v.as_ptr() as *const i8);
            lens.push(v.len() as c_uint);
        }

        unsafe {
            ffi::hs_scan_vector(
                self.as_ptr(),
                ptrs.as_slice().as_ptr() as *const *const i8,
                lens.as_slice().as_ptr() as *const _,
                data.len() as u32,
                0,
                scratch.as_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            )
            .ok()
            .map(|_| ())
        }
    }
}

impl Stream {
    /// pattern matching takes place for stream-mode pattern databases.
    pub fn scan<T, D>(
        &self,
        data: T,
        scratch: &ScratchRef,
        callback: MatchEventCallback<D>,
        context: Option<&D>,
    ) -> Result<(), Error>
    where
        T: AsRef<[u8]>,
    {
        let bytes = data.as_ref();

        unsafe {
            ffi::hs_scan_stream(
                self.as_ptr(),
                bytes.as_ptr() as *const i8,
                bytes.len() as u32,
                0,
                scratch.as_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            )
            .ok()
            .map(|_| ())
        }
    }
}
