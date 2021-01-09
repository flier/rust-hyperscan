use std::mem::MaybeUninit;

use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::{
    common::{DatabaseRef, Streaming},
    error::AsResult,
    ffi,
    runtime::{MatchEventHandler, ScratchRef},
    Result,
};

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
    pub unsafe type Stream: Send {
        type CType = ffi::hs_stream_t;

        fn drop = drop_stream;
        fn clone = clone_stream;
    }
}

fn drop_stream(_s: *mut ffi::hs_stream_t) {}

/// Duplicate the given stream.
///
/// The new stream will have the same state as the original including the current stream offset.
unsafe fn clone_stream(s: *mut ffi::hs_stream_t) -> *mut ffi::hs_stream_t {
    let mut p = MaybeUninit::uninit();

    ffi::hs_copy_stream(p.as_mut_ptr(), s).expect("copy stream");

    p.assume_init()
}

impl StreamRef {
    /// Reset a stream to an initial state.
    ///
    /// Conceptually, this is equivalent to performing `Stream::close` on the given stream,
    /// followed by `StreamingDatabase::open_stream`.
    /// This new stream replaces the original stream in memory,
    /// avoiding the overhead of freeing the old stream and allocating the new one.
    ///
    /// Note: This operation may result in matches being returned (via calls to the match event callback)
    /// for expressions anchored to the end of the original data stream
    /// (for example, via the use of the `$` meta-character).
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
    /// for d in &data {
    ///     st.scan(d, &s, &mut callback).unwrap();
    /// }
    ///
    /// st.reset(&s, &mut callback).unwrap();
    ///
    /// for d in &data {
    ///     st.scan(d, &s, &mut callback).unwrap();
    /// }
    ///
    /// st.close(&s, callback).unwrap();
    ///
    /// assert_eq!(matches, vec![(4, 8), (4, 8)]);
    /// ```
    pub fn reset<F>(&self, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        F: MatchEventHandler,
    {
        unsafe {
            let (callback, userdata) = on_match_event.split();

            ffi::hs_reset_stream(self.as_ptr(), 0, scratch.as_ptr(), callback, userdata).ok()
        }
    }

    /// Duplicate the given `from` stream state onto the stream.
    ///
    /// The stream will first be reset (reporting any EOD matches if a `on_match_event` callback handler is provided).
    ///
    /// Note: the stream and the `from` stream must be open against the same database.
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
    /// let mut matches = vec![];
    ///
    /// let mut callback = |_, from, to, _| {
    ///     matches.push((from, to));
    ///
    ///     Matching::Continue
    /// };
    ///
    /// st.scan("foo t", &s, &mut callback).unwrap();
    /// st.scan("es", &s, &mut callback).unwrap();
    ///
    /// let st2 = db.open_stream().unwrap();
    ///
    /// st2.scan("test", &s, &mut callback).unwrap();
    /// st2.reset_and_copy_stream(&st, &s, &mut callback).unwrap();
    /// st2.scan("t bar", &s, &mut callback).unwrap();
    /// st2.close(&s, &mut callback).unwrap();
    ///
    /// st.close(&s, Matching::Terminate).unwrap();
    ///
    /// assert_eq!(matches, vec![(0, 4), (4, 8)]);
    /// ```
    pub fn reset_and_copy_stream<F>(&self, from: &StreamRef, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        F: MatchEventHandler,
    {
        unsafe {
            let (callback, userdata) = on_match_event.split();

            ffi::hs_reset_and_copy_stream(self.as_ptr(), from.as_ptr(), scratch.as_ptr(), callback, userdata).ok()
        }
    }
}

impl Stream {
    /// Close a stream.
    ///
    /// This function completes matching on the given stream and frees the memory associated with the stream state.
    /// After this call, the stream is invalid and can no longer be used.
    /// To reuse the stream state after completion, rather than closing it, the `StreamRef::reset` function can be used.
    ///
    /// This function must be called for any stream created with `StreamingDatabase::open_stream`,
    /// even if scanning has been terminated by a non-zero return from the match callback function.
    pub fn close<F>(self, scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        F: MatchEventHandler,
    {
        unsafe {
            let (callback, userdata) = on_match_event.split();

            ffi::hs_close_stream(self.as_ptr(), scratch.as_ptr(), callback, userdata).ok()
        }
    }
}

impl StreamRef {
    /// Creates a compressed representation of the provided stream in the buffer provided.
    ///
    /// This compressed representation can be converted back into a stream state by using `expand()`
    /// or `reset_and_expand()`.
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
    /// let mut matches = vec![];
    ///
    /// let mut callback = |_, from, to, _| {
    ///     matches.push((from, to));
    ///
    ///     Matching::Continue
    /// };
    ///
    /// st.scan("foo t", &s, &mut callback).unwrap();
    /// st.scan("es", &s, &mut callback).unwrap();
    ///
    /// let mut buf = [0; 8192];
    /// let len = st.compress(&mut buf).unwrap();
    /// st.close(&s, Matching::Terminate).unwrap();
    ///
    /// let st2 = db.expand_stream(&buf[..len]).unwrap();
    /// st2.scan("t bar", &s, &mut callback).unwrap();
    /// st2.close(&s, &mut callback).unwrap();
    ///
    /// assert_eq!(matches, vec![(4, 8)]);
    /// ```
    pub fn compress(&self, buf: &mut [u8]) -> Result<usize> {
        let mut size = MaybeUninit::uninit();

        unsafe {
            ffi::hs_compress_stream(self.as_ptr(), buf.as_mut_ptr() as *mut _, buf.len(), size.as_mut_ptr())
                .ok()
                .map(|_| size.assume_init())
        }
    }

    /// Decompresses a compressed representation created by `StreamRef::compress` on top of the stream.
    /// The stream will first be reset (reporting any EOD matches).
    ///
    /// Note: the stream must be opened against the same database as the compressed stream.
    ///
    /// Note: `buf` must correspond to a complete compressed representation created by `StreamRef::compress` of a stream
    /// that was opened against `db`. It is not always possible to detect misuse of this API and behaviour is undefined
    /// if these properties are not satisfied.
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
    /// let mut matches = vec![];
    ///
    /// let mut callback = |_, from, to, _| {
    ///     matches.push((from, to));
    ///
    ///     Matching::Continue
    /// };
    ///
    /// st.scan("foo t", &s, &mut callback).unwrap();
    /// st.scan("es", &s, &mut callback).unwrap();
    ///
    /// let mut buf = [0; 8192];
    /// let len = st.compress(&mut buf).unwrap();
    /// st.scan("t bar", &s, &mut callback).unwrap();
    ///
    /// st.reset_and_expand(&buf[..len], &s, &mut callback).unwrap();
    /// st.scan("t bar", &s, &mut callback).unwrap();
    /// st.close(&s, &mut callback).unwrap();
    ///
    /// assert_eq!(matches, vec![(4, 8), (4, 8)]);
    /// ```
    pub fn reset_and_expand<F>(&self, buf: &[u8], scratch: &ScratchRef, mut on_match_event: F) -> Result<()>
    where
        F: MatchEventHandler,
    {
        unsafe {
            let (callback, userdata) = on_match_event.split();

            ffi::hs_reset_and_expand_stream(
                self.as_ptr(),
                buf.as_ptr() as *const _,
                buf.len(),
                scratch.as_ptr(),
                callback,
                userdata,
            )
            .ok()
        }
    }
}

impl DatabaseRef<Streaming> {
    /// Decompresses a compressed representation created by `StreamRef::compress()` into a new stream.
    ///
    /// Note: `buf` must correspond to a complete compressed representation created by `StreamRef::compress()` of a stream
    /// that was opened against `db`. It is not always possible to detect misuse of this API and behaviour is undefined
    /// if these properties are not satisfied.
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
    /// let mut matches = vec![];
    ///
    /// let mut callback = |_, from, to, _| {
    ///     matches.push((from, to));
    ///
    ///     Matching::Continue
    /// };
    ///
    /// st.scan("foo t", &s, &mut callback).unwrap();
    /// st.scan("es", &s, &mut callback).unwrap();
    ///
    /// let mut buf = [0; 8192];
    /// let len = st.compress(&mut buf).unwrap();
    /// st.close(&s, Matching::Terminate).unwrap();
    ///
    /// let st2 = db.expand_stream(&buf[..len]).unwrap();
    /// st2.scan("t bar", &s, &mut callback).unwrap();
    /// st2.close(&s, &mut callback).unwrap();
    ///
    /// assert_eq!(matches, vec![(4, 8)]);
    /// ```
    pub fn expand_stream(&self, buf: &[u8]) -> Result<Stream> {
        let mut stream = MaybeUninit::uninit();

        unsafe {
            ffi::hs_expand_stream(self.as_ptr(), stream.as_mut_ptr(), buf.as_ptr() as *const _, buf.len())
                .ok()
                .map(|_| Stream::from_ptr(stream.assume_init()))
        }
    }
}
