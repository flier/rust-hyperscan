use std::mem::{self, MaybeUninit};

use anyhow::Result;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::common::{DatabaseRef, Streaming};
use crate::errors::AsResult;
use crate::ffi;
use crate::runtime::{split_closure, Matching, ScratchRef};

impl DatabaseRef<Streaming> {
    /// Provides the size of the stream state allocated by a single stream opened against the given database.
    pub fn stream_size(&self) -> Result<usize> {
        let mut size = MaybeUninit::uninit();

        unsafe { ffi::hs_stream_size(self.as_ptr(), size.as_mut_ptr()).map(|_| size.assume_init()) }
    }

    /// Open and initialise a stream.
    pub fn open_stream(&self) -> Result<Stream> {
        let mut s = MaybeUninit::uninit();

        unsafe { ffi::hs_open_stream(self.as_ptr(), 0, s.as_mut_ptr()).map(|_| Stream::from_ptr(s.assume_init())) }
    }
}

foreign_type! {
    /// A pattern matching state can be maintained across multiple blocks of target data
    pub unsafe type Stream {
        type CType = ffi::hs_stream_t;

        fn drop = drop_stream;
        fn clone = clone_stream;
    }
}

fn drop_stream(_s: *mut ffi::hs_stream_t) {}

unsafe fn clone_stream(s: *mut ffi::hs_stream_t) -> *mut ffi::hs_stream_t {
    let mut p = MaybeUninit::uninit();

    ffi::hs_copy_stream(p.as_mut_ptr(), s).expect("copy stream");

    p.assume_init()
}

impl StreamRef {
    /// Reset a stream to an initial state.
    pub fn reset<F>(&self, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        F: FnMut(u32, u64, u64, u32) -> Matching,
    {
        let (callback, userdata) = unsafe { split_closure(&mut on_match_event) };

        unsafe {
            ffi::hs_reset_stream(
                self.as_ptr(),
                0,
                scratch.as_ptr(),
                Some(mem::transmute(callback)),
                userdata,
            )
            .ok()
        }
    }
}

impl Stream {
    /// Close a stream.
    pub fn close<F>(self, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        F: FnMut(u32, u64, u64, u32) -> Matching,
    {
        let (callback, userdata) = unsafe { split_closure(&mut on_match_event) };

        unsafe {
            ffi::hs_close_stream(
                self.as_ptr(),
                scratch.as_ptr(),
                Some(mem::transmute(callback)),
                userdata,
            )
            .ok()
        }
    }
}
