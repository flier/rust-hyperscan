use std::io::Read;
use std::mem;
use std::ptr;

use foreign_types::ForeignTypeRef;
use libc::{c_char, c_uint};

use crate::{
    common::{Block, DatabaseRef, Streaming, Vectored},
    error::AsResult,
    ffi,
    runtime::{split_closure, ScratchRef, StreamRef},
    Result,
};

#[cfg(feature = "async")]
use futures::io::{AsyncRead, AsyncReadExt};

/// Indicating whether or not matching should continue on the target data.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Matching {
    /// The matching should continue
    Continue = 0,
    /// The matching should cease
    Terminate = 1,
}

impl Default for Matching {
    fn default() -> Self {
        Matching::Continue
    }
}

/// Definition of the match event callback function type.
///
/// A callback function matching the defined type must be provided by the
/// application calling the `DatabaseRef::scan` or `StreamRef::scan` functions
/// (or other streaming calls which can produce matches).
///
/// This callback function will be invoked whenever a match is located in the
/// target data during the execution of a scan. The details of the match are
/// passed in as parameters to the callback function, and the callback function
/// should return a value indicating whether or not matching should continue on
/// the target data. If no callbacks are desired from a scan call, NULL may be
/// provided in order to suppress match production.
///
/// This callback function should not attempt to call Hyperscan API functions on
/// the same stream nor should it attempt to reuse the scratch space allocated
/// for the API calls that caused it to be triggered. Making another call to the
/// Hyperscan library with completely independent parameters should work (for
/// example, scanning a different database in a new stream and with new scratch
/// space), but reusing data structures like stream state and/or scratch space
/// will produce undefined behavior.
pub trait MatchEventHandler {
    /// Split the match event handler to callback and userdata.
    ///
    /// # Safety
    ///
    /// Do not implement this trait directly, use `()`, `Matching` or `|id, from, to, flags| -> Matching`.
    unsafe fn split(&mut self) -> (ffi::match_event_handler, *mut libc::c_void);
}

impl MatchEventHandler for () {
    unsafe fn split(&mut self) -> (ffi::match_event_handler, *mut libc::c_void) {
        (None, ptr::null_mut())
    }
}

impl MatchEventHandler for Matching {
    unsafe fn split(&mut self) -> (ffi::match_event_handler, *mut libc::c_void) {
        unsafe extern "C" fn trampoline(_: u32, _: u64, _: u64, _: u32, ctx: *mut ::libc::c_void) -> ::libc::c_int {
            ctx.cast::<Matching>().read() as _
        }

        (Some(trampoline), self as *mut _ as *mut _)
    }
}

impl MatchEventHandler for (ffi::match_event_handler, *mut libc::c_void) {
    unsafe fn split(&mut self) -> (ffi::match_event_handler, *mut libc::c_void) {
        *self
    }
}

impl<F> MatchEventHandler for F
where
    F: FnMut(u32, u64, u64, u32) -> Matching,
{
    unsafe fn split(&mut self) -> (ffi::match_event_handler, *mut libc::c_void) {
        let (callback, userdata) = split_closure(self);

        (Some(mem::transmute(callback)), userdata)
    }
}

impl DatabaseRef<Block> {
    /// The block (non-streaming) regular expression scanner.
    ///
    /// This is the function call in which the actual pattern matching takes place for block-mode pattern databases.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hyperscan::prelude::*;
    /// let db: BlockDatabase = pattern! {"test"; CASELESS | SOM_LEFTMOST}.build().unwrap();
    /// let s = db.alloc_scratch().unwrap();
    /// let mut matches = vec![];
    ///
    /// db.scan("foo test bar", &s, |_, from, to, _| {
    ///     matches.push(from..to);
    ///     Matching::Continue
    /// }).unwrap();
    ///
    /// assert_eq!(matches, vec![4..8]);
    /// ```
    pub fn scan<T, F>(&self, data: T, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        T: AsRef<[u8]>,
        F: MatchEventHandler,
    {
        let data = data.as_ref();

        unsafe {
            let (callback, userdata) = on_match_event.split();

            ffi::hs_scan(
                self.as_ptr(),
                data.as_ptr() as *const c_char,
                data.len() as u32,
                0,
                scratch.as_ptr(),
                callback,
                userdata,
            )
            .ok()
        }
    }
}

impl DatabaseRef<Vectored> {
    /// The vectored regular expression scanner.
    ///
    /// This is the function call in which the actual pattern matching takes place for vectoring-mode pattern databases.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hyperscan::prelude::*;
    /// let db: VectoredDatabase = pattern!{"test"; CASELESS|SOM_LEFTMOST}.build().unwrap();
    /// let s = db.alloc_scratch().unwrap();
    ///
    /// let mut matches = vec![];
    ///
    /// db.scan(vec!["foo", "test", "bar"], &s, |id, from, to, _| {
    ///     matches.push(from..to);
    ///     Matching::Continue
    /// }).unwrap();
    ///
    /// assert_eq!(matches, vec![3..7]);
    /// ```
    pub fn scan<I, T, F>(&self, data: I, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<[u8]>,
        F: MatchEventHandler,
    {
        let (ptrs, lens): (Vec<_>, Vec<_>) = data
            .into_iter()
            .map(|buf| {
                let buf = buf.as_ref();

                (buf.as_ptr() as *const i8, buf.len() as c_uint)
            })
            .unzip();

        unsafe {
            let (callback, userdata) = on_match_event.split();

            ffi::hs_scan_vector(
                self.as_ptr(),
                ptrs.as_slice().as_ptr() as *const *const c_char,
                lens.as_slice().as_ptr() as *const _,
                ptrs.len() as u32,
                0,
                scratch.as_ptr(),
                callback,
                userdata,
            )
            .ok()
        }
    }
}

const SCAN_BUF_SIZE: usize = 4096;

impl DatabaseRef<Streaming> {
    /// Pattern matching takes place for stream-mode pattern databases.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::io::Cursor;
    /// # use hyperscan::prelude::*;
    /// const SCAN_BUF_SIZE: usize = 4096;
    /// let mut buf = String::from_utf8(vec![b'x'; SCAN_BUF_SIZE - 2]).unwrap();
    ///
    /// buf.push_str("baaab");
    ///
    /// let db: StreamingDatabase = pattern! { "a+"; SOM_LEFTMOST }.build().unwrap();
    /// let s = db.alloc_scratch().unwrap();
    /// let mut cur = Cursor::new(buf.as_bytes());
    /// let mut matches = vec![];
    ///
    /// db.scan(&mut cur, &s, |_, from, to, _| {
    ///     matches.push((from, to));
    ///
    ///     Matching::Continue
    /// })
    /// .unwrap();
    ///
    /// assert_eq!(matches, vec![(4095, 4096), (4095, 4097), (4095, 4098)]);
    /// ```
    pub fn scan<R, F>(&self, reader: &mut R, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        R: Read,
        F: MatchEventHandler,
    {
        let stream = self.open_stream()?;
        let mut buf = [0; SCAN_BUF_SIZE];

        let (callback, userdata) = unsafe { on_match_event.split() };

        while let Ok(len) = reader.read(&mut buf[..]) {
            if len == 0 {
                break;
            }

            stream.scan(&buf[..len], scratch, (callback, userdata))?;
        }

        stream.close(scratch, (callback, userdata))
    }

    /// Pattern matching takes place for stream-mode pattern databases using AsyncRead.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use futures::io::Cursor;
    /// # use hyperscan::prelude::*;
    /// # use tokio_test;
    /// const SCAN_BUF_SIZE: usize = 4096;
    /// let mut buf = String::from_utf8(vec![b'x'; SCAN_BUF_SIZE - 2]).unwrap();
    ///
    /// buf.push_str("baaab");
    ///
    /// let db: StreamingDatabase = pattern! { "a+"; SOM_LEFTMOST }.build().unwrap();
    /// let s = db.alloc_scratch().unwrap();
    /// let mut cur = Cursor::new(buf.as_bytes());
    /// let mut matches = vec![];
    ///
    /// tokio_test::block_on(async {
    ///     db.async_scan(&mut cur, &s, |_, from, to, _| {
    ///         matches.push((from, to));
    ///
    ///         Matching::Continue
    ///     }).await.unwrap();
    /// });
    ///
    /// assert_eq!(matches, vec![(4095, 4096), (4095, 4097), (4095, 4098)]);
    /// ```
    #[cfg(feature = "async")]
    pub async fn async_scan<R, F>(&self, reader: &mut R, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        R: AsyncRead + Unpin,
        F: MatchEventHandler,
    {
        let stream = self.open_stream()?;
        let mut buf = [0; SCAN_BUF_SIZE];

        let (callback, userdata) = unsafe { on_match_event.split() };

        while let Ok(len) = reader.read(&mut buf[..]).await {
            if len == 0 {
                break;
            }

            stream.scan(&buf[..len], scratch, (callback, userdata))?;
        }

        stream.close(scratch, (callback, userdata))
    }
}

impl StreamRef {
    /// Write data to be scanned to the opened stream.
    ///
    /// This is the function call in which the actual pattern matching takes place as data is written to the stream.
    /// Matches will be returned via the `on_match_event` callback supplied.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use hyperscan::prelude::*;
    /// let db: StreamingDatabase = pattern! {"test"; SOM_LEFTMOST}.build().unwrap();
    ///
    /// let s = db.alloc_scratch().unwrap();
    /// let st = db.open_stream().unwrap();
    ///
    /// let data = vec!["foo t", "es", "t bar"];
    /// let mut matches = vec![];
    ///
    /// let mut callback = |_, from, to, _| {
    ///     matches.push((from, to));
    ///
    ///     Matching::Continue
    /// };
    ///
    /// for d in data {
    ///     st.scan(d, &s, &mut callback).unwrap();
    /// }
    ///
    /// st.close(&s, callback).unwrap();
    ///
    /// assert_eq!(matches, vec![(4, 8)]);
    /// ```
    pub fn scan<T, F>(&self, data: T, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        T: AsRef<[u8]>,
        F: MatchEventHandler,
    {
        let data = data.as_ref();

        unsafe {
            let (callback, userdata) = on_match_event.split();

            ffi::hs_scan_stream(
                self.as_ptr(),
                data.as_ptr() as *const c_char,
                data.len() as u32,
                0,
                scratch.as_ptr(),
                callback,
                userdata,
            )
            .ok()
        }
    }
}
