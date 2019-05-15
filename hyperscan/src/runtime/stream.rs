use core::mem;
use core::ptr::null_mut;

use failure::Error;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::common::{Database, Streaming};
use crate::errors::AsResult;
use crate::runtime::{MatchEventCallback, ScratchRef};

impl Database<Streaming> {
    pub fn stream_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;

        unsafe { ffi::hs_stream_size(self.as_ptr(), &mut size).ok().map(|_| size) }
    }
}

impl Database<Streaming> {
    /// Open and initialise a stream.
    pub fn open_stream(&self) -> Result<Stream, Error> {
        let mut s = null_mut();

        unsafe {
            ffi::hs_open_stream(self.as_ptr(), 0, &mut s)
                .ok()
                .map(|_| Stream::from_ptr(s))
        }
    }
}

foreign_type! {
    /// A pattern matching state can be maintained across multiple blocks of target data
    pub type Stream {
        type CType = ffi::hs_stream_t;

        fn drop = drop_stream;
        fn clone = clone_stream;
    }
}

fn drop_stream(_s: *mut ffi::hs_stream_t) {}

unsafe fn clone_stream(s: *mut ffi::hs_stream_t) -> *mut ffi::hs_stream_t {
    let mut p = null_mut();

    ffi::hs_copy_stream(&mut p, s).ok().unwrap();

    p
}

impl StreamRef {
    pub fn reset<D>(
        &self,
        scratch: &ScratchRef,
        callback: MatchEventCallback<D>,
        context: Option<&D>,
    ) -> Result<(), Error> {
        unsafe {
            ffi::hs_reset_stream(
                self.as_ptr(),
                0,
                scratch.as_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            )
            .ok()
        }
    }
}

impl Stream {
    pub fn close<D>(
        self,
        scratch: &ScratchRef,
        callback: MatchEventCallback<D>,
        context: Option<&D>,
    ) -> Result<(), Error> {
        unsafe {
            ffi::hs_close_stream(
                self.as_ptr(),
                scratch.as_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            )
            .ok()
        }
    }
}
