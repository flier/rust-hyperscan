use std::ptr;
use std::mem;
use std::os::raw::c_void;
use std::ops::{Deref, DerefMut, Fn};

use raw::*;
use errors::Error;
use common::{BlockDatabase, VectoredDatabase, StreamingDatabase};

pub trait Scratch {
    fn size(&self) -> Result<usize, Error>;

    fn realloc(&mut self, db: *const hs_database_t) -> Result<&Self, Error>;
}

pub struct RawScratch(*mut hs_scratch_t);

impl RawScratch {
    pub fn alloc(db: *const hs_database_t) -> Result<RawScratch, Error> {
        let mut s: *mut hs_scratch_t = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_alloc_scratch(db, &mut s));
        }

        Result::Ok(RawScratch(s))
    }
}

impl Drop for RawScratch {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            assert_hs_error!(hs_free_scratch(self.0));
        }
    }
}

impl Clone for RawScratch {
    fn clone(&self) -> Self {
        let mut s: *mut hs_scratch_t = ptr::null_mut();

        unsafe {
            assert_hs_error!(hs_clone_scratch(self.0, &mut s));
        }

        RawScratch(s)
    }
}

impl Deref for RawScratch {
    type Target = *mut hs_scratch_t;

    #[inline]
    fn deref(&self) -> &*mut hs_scratch_t {
        &self.0
    }
}

impl Scratch for RawScratch {
    fn size(&self) -> Result<usize, Error> {
        let mut size: size_t = 0;

        unsafe {
            check_hs_error!(hs_scratch_size(self.0, &mut size));
        }

        Result::Ok(size as usize)
    }

    fn realloc(&mut self, db: *const hs_database_t) -> Result<&Self, Error> {
        unsafe {
            check_hs_error!(hs_alloc_scratch(db, &mut self.0));
        }

        Result::Ok(self)
    }
}

pub type MatchEventCallback = Fn(u32, u64, u64, u32) -> bool;

const TRUE: int32_t = 1;
const FALSE: int32_t = 0;

unsafe extern "C" fn match_event_callback(id: uint32_t,
                                          from: uint64_t,
                                          to: uint64_t,
                                          flags: uint32_t,
                                          context: *mut c_void)
                                          -> int32_t {

    let callback: &*const MatchEventCallback = mem::transmute(context);

    if (**callback)(id as u32, from as u64, to as u64, flags as u32) {
        TRUE
    } else {
        FALSE
    }
}

pub trait Scannable {
    fn as_bytes(&self) -> &[u8];
}

impl<'a> Scannable for &'a [u8] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        &self
    }
}

impl<'a> Scannable for &'a str {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        str::as_bytes(self)
    }
}
impl<'a> Scannable for &'a String {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

pub trait BlockScanner<T: Scannable> {
    fn scan(&self,
            data: T,
            scratch: &RawScratch,
            handler: Option<&MatchEventCallback>)
            -> Result<&Self, Error>;
}

pub trait VectoredScanner<T: Scannable> {
    fn scan(&self,
            data: &Vec<T>,
            scratch: &RawScratch,
            handler: Option<&MatchEventCallback>)
            -> Result<&Self, Error>;
}

pub type StreamFlags = u32;

pub trait Stream {
    fn close(&self,
             scratch: &RawScratch,
             handler: Option<&MatchEventCallback>)
             -> Result<&Self, Error>;

    fn reset(&self,
             flags: StreamFlags,
             scratch: &RawScratch,
             handler: Option<&MatchEventCallback>)
             -> Result<&Self, Error>;
}

pub trait StreamingScanner<S: Stream> {
    fn open_stream(&self, flags: StreamFlags) -> Result<S, Error>;
}

struct WrappedMatchEventHandler {
    handler: match_event_handler,
    context: *mut c_void,
}

macro_rules! wrap_match_event_handler {
    ($handler:ident) => (match $handler {
        None => {
            WrappedMatchEventHandler{ handler: Option::None, context: ptr::null_mut()}
        }
        Some(callback) => {
            WrappedMatchEventHandler{ handler: Option::Some(match_event_callback), context: mem::transmute(&callback)}
        }
    })
}

impl<T: Scannable> BlockScanner<T> for BlockDatabase {
    #[inline]
    fn scan(&self,
            data: T,
            scratch: &RawScratch,
            handler: Option<&MatchEventCallback>)
            -> Result<&Self, Error> {
        unsafe {
            let w = wrap_match_event_handler!(handler);
            let bytes = data.as_bytes();

            check_hs_error!(hs_scan(**self,
                                    bytes.as_ptr() as *const i8,
                                    bytes.len() as u32,
                                    0 as u32,
                                    scratch.0,
                                    w.handler,
                                    w.context));
        }

        Result::Ok(&self)
    }
}

impl<T: Scannable> VectoredScanner<T> for VectoredDatabase {
    #[inline]
    fn scan(&self,
            data: &Vec<T>,
            scratch: &RawScratch,
            handler: Option<&MatchEventCallback>)
            -> Result<&Self, Error> {

        let mut ptrs = Vec::with_capacity(data.len());
        let mut lens = Vec::with_capacity(data.len());

        for d in data.iter() {
            let bytes = d.as_bytes();
            ptrs.push(bytes.as_ptr() as *const i8);
            lens.push(bytes.len() as uint32_t);
        }

        unsafe {
            let w = wrap_match_event_handler!(handler);

            check_hs_error!(hs_scan_vector(**self,
                                           ptrs.as_slice().as_ptr() as *const *const i8,
                                           lens.as_slice().as_ptr() as *const uint32_t,
                                           data.len() as u32,
                                           0 as u32,
                                           scratch.0,
                                           w.handler,
                                           w.context));
        }

        Result::Ok(&self)
    }
}

impl StreamingScanner<RawStream> for StreamingDatabase {
    fn open_stream(&self, flags: StreamFlags) -> Result<RawStream, Error> {
        let mut id: *mut hs_stream_t = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_open_stream(**self, flags, &mut id));
        }

        Result::Ok(RawStream(id))
    }
}

pub struct RawStream(*mut hs_stream_t);

impl Deref for RawStream {
    type Target = *mut hs_stream_t;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RawStream {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Clone for RawStream {
    fn clone(&self) -> Self {
        let mut id: *mut hs_stream_t = ptr::null_mut();

        unsafe {
            assert_hs_error!(hs_copy_stream(&mut id, self.0));
        }

        RawStream(id)
    }
}

impl Stream for RawStream {
    fn close(&self,
             scratch: &RawScratch,
             handler: Option<&MatchEventCallback>)
             -> Result<&Self, Error> {
        unsafe {
            let w = wrap_match_event_handler!(handler);

            check_hs_error!(hs_close_stream(self.0, scratch.0, w.handler, w.context));
        }

        Result::Ok(&self)
    }

    fn reset(&self,
             flags: StreamFlags,
             scratch: &RawScratch,
             handler: Option<&MatchEventCallback>)
             -> Result<&Self, Error> {
        unsafe {
            let w = wrap_match_event_handler!(handler);

            check_hs_error!(hs_reset_stream(self.0, flags, scratch.0, w.handler, w.context));
        }

        Result::Ok(&self)
    }
}

impl<T: Scannable> BlockScanner<T> for RawStream {
    #[inline]
    fn scan(&self,
            data: T,
            scratch: &RawScratch,
            handler: Option<&MatchEventCallback>)
            -> Result<&Self, Error> {
        unsafe {
            let w = wrap_match_event_handler!(handler);
            let bytes = data.as_bytes();

            check_hs_error!(hs_scan_stream(self.0,
                                           bytes.as_ptr() as *const i8,
                                           bytes.len() as u32,
                                           0 as u32,
                                           scratch.0,
                                           w.handler,
                                           w.context));
        }

        Result::Ok(&self)
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;

    use super::super::*;

    #[test]
    fn test_scratch() {
        let db: BlockDatabase = pattern!{"test"}.build().unwrap();

        assert!(*db != ptr::null_mut());

        let s = RawScratch::alloc(*db).unwrap();

        assert!(*s != ptr::null_mut());

        assert_eq!(s.size().unwrap(), 2385);

        let mut s2 = s.clone();

        assert!(*s2 != ptr::null_mut());

        assert_eq!(s2.size().unwrap(), 2385);

        let db2: VectoredDatabase = pattern!{"foobar"}.build().unwrap();

        assert!(s2.realloc(*db2).unwrap().size().unwrap() > s.size().unwrap());
    }

    #[test]
    fn test_block_scan() {
        let db: BlockDatabase = pattern!{"test", flags => HS_FLAG_CASELESS|HS_FLAG_SOM_LEFTMOST}
                                    .build()
                                    .unwrap();
        let s = RawScratch::alloc(*db).unwrap();

        db.scan("foo test bar", &s, Option::None).unwrap();

        let callback = |id: u32, from: u64, to: u64, flags: u32| {
            assert_eq!(id, 0);
            assert_eq!(from, 4);
            assert_eq!(to, 8);
            assert_eq!(flags, 0);

            true
        };

        assert_eq!(db.scan("foo test bar".as_bytes(), &s, Option::Some(&callback))
                     .err(),
                   Some(Error::ScanTerminated));
    }

    #[test]
    fn test_vectored_scan() {
        let db: VectoredDatabase = pattern!{"test", flags => HS_FLAG_CASELESS|HS_FLAG_SOM_LEFTMOST}
                                       .build()
                                       .unwrap();
        let s = RawScratch::alloc(*db).unwrap();

        let data = vec!["foo", "test", "bar"];

        db.scan(&data, &s, Option::None).unwrap();

        let callback = |id: u32, from: u64, to: u64, flags: u32| {
            assert_eq!(id, 0);
            assert_eq!(from, 3);
            assert_eq!(to, 7);
            assert_eq!(flags, 0);

            true
        };

        let data = vec!["foo".as_bytes(), "test".as_bytes(), "bar".as_bytes()];

        assert_eq!(db.scan(&data, &s, Option::Some(&callback))
                     .err(),
                   Some(Error::ScanTerminated));
    }

    #[test]
    fn test_streaming_scan() {
        let db: StreamingDatabase = pattern!{"test", flags => HS_FLAG_CASELESS}.build().unwrap();

        let s = RawScratch::alloc(*db).unwrap();
        let st = db.open_stream(0).unwrap();

        let data = vec!["foo", "test", "bar"];
        let callback = |id: u32, from: u64, to: u64, flags: u32| {
            assert_eq!(id, 0);
            assert_eq!(from, 0);
            assert_eq!(to, 7);
            assert_eq!(flags, 0);

            false
        };

        for d in data {
            st.scan(d, &s, Option::Some(&callback))
              .unwrap();
        }

        st.close(&s, Option::Some(&callback))
          .unwrap();

    }
}
