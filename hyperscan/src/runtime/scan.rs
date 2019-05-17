use core::mem;
use core::ops::Deref;
use core::ptr::null_mut;
use std::io::Read;

use failure::Error;
use foreign_types::ForeignTypeRef;
use libc::c_uint;

use crate::common::{Block, DatabaseRef, Streaming, Vectored};
use crate::errors::AsResult;
use crate::ffi;
use crate::runtime::{ScratchRef, StreamRef};

/// Scannable buffer
pub trait Scannable: AsRef<[u8]> {}

impl<T> Scannable for T where T: AsRef<[u8]> {}

/// Definition of the match event callback function type.
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
///
/// Fn(id: u32, from: u64, to: u64, flags: u32) -> bool
///
pub type MatchEventCallback<P> = Option<fn(id: u32, from: u64, to: u64, flags: u32, context: Option<P>) -> u32>;

impl DatabaseRef<Block> {
    /// pattern matching takes place for block-mode pattern databases.
    pub fn scan<'a, T, P>(
        &self,
        data: T,
        scratch: &ScratchRef,
        callback: MatchEventCallback<P>,
        context: Option<P>,
    ) -> Result<(), Error>
    where
        T: Scannable,
        P: Deref,
        P::Target: Sized,
    {
        let data = data.as_ref();

        unsafe {
            ffi::hs_scan(
                self.as_ptr(),
                data.as_ptr() as *const i8,
                data.len() as u32,
                0,
                scratch.as_ptr(),
                mem::transmute(callback),
                context.map_or_else(null_mut, |p| &*p as *const _ as *mut _),
            )
            .ok()
        }
    }
}

impl DatabaseRef<Vectored> {
    /// pattern matching takes place for vectoring-mode pattern databases.
    pub fn scan<'a, I, T, P>(
        &self,
        data: I,
        scratch: &ScratchRef,
        callback: MatchEventCallback<P>,
        context: Option<P>,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = T>,
        T: Scannable,
        P: Deref,
        P::Target: Sized,
    {
        let (ptrs, lens): (Vec<_>, Vec<_>) = data
            .into_iter()
            .map(|buf| {
                let buf = buf.as_ref();

                (buf.as_ptr() as *const i8, buf.len() as c_uint)
            })
            .unzip();

        unsafe {
            ffi::hs_scan_vector(
                self.as_ptr(),
                ptrs.as_slice().as_ptr() as *const *const i8,
                lens.as_slice().as_ptr() as *const _,
                ptrs.len() as u32,
                0,
                scratch.as_ptr(),
                mem::transmute(callback),
                context.map_or_else(null_mut, |p| &*p as *const _ as *mut _),
            )
            .ok()
        }
    }
}

const SCAN_BUF_SIZE: usize = 4096;

impl DatabaseRef<Streaming> {
    /// pattern matching takes place for stream-mode pattern databases.
    pub fn scan<'a, R, P>(
        &self,
        reader: &mut R,
        scratch: &ScratchRef,
        callback: MatchEventCallback<P>,
        context: Option<P>,
    ) -> Result<(), Error>
    where
        R: Read,
        P: Deref + Copy,
        P::Target: Sized,
    {
        let stream = self.open_stream()?;
        let mut buf = [0; SCAN_BUF_SIZE];

        while let Ok(len) = reader.read(&mut buf[..]) {
            if len == 0 {
                break;
            }

            stream.scan(&buf[..len], scratch, callback, context)?;
        }

        stream.close(scratch, callback, context)
    }
}

impl StreamRef {
    /// pattern matching takes place for stream-mode pattern databases.
    pub fn scan<'a, T, P>(
        &self,
        data: T,
        scratch: &ScratchRef,
        callback: MatchEventCallback<P>,
        context: Option<P>,
    ) -> Result<(), Error>
    where
        T: Scannable,
        P: Deref,
        P::Target: Sized,
    {
        let data = data.as_ref();

        unsafe {
            ffi::hs_scan_stream(
                self.as_ptr(),
                data.as_ptr() as *const i8,
                data.len() as u32,
                0,
                scratch.as_ptr(),
                mem::transmute(callback),
                context.map_or_else(null_mut, |p| &*p as *const _ as *mut _),
            )
            .ok()
        }
    }
}

#[cfg(test)]
pub mod tests {
    use core::cell::RefCell;
    use core::pin::Pin;
    use std::io::Cursor;

    use super::*;
    use crate::common::*;
    use crate::compile::Builder;
    use crate::errors::HsError;

    #[test]
    fn test_block_scan() {
        let _ = pretty_env_logger::try_init();

        let db: BlockDatabase = pattern! {"test"; CASELESS | SOM_LEFTMOST}.build().unwrap();
        let s = db.alloc().unwrap();

        db.scan::<_, &()>("foo test bar", &s, None, None).unwrap();

        fn callback<T>(id: u32, from: u64, to: u64, flags: u32, _: T) -> u32 {
            assert_eq!(id, 0);
            assert_eq!(from, 4);
            assert_eq!(to, 8);
            assert_eq!(flags, 0);

            1
        };

        assert_eq!(
            db.scan::<_, &()>("foo test bar".as_bytes(), &s, Some(callback), None)
                .err()
                .unwrap()
                .downcast_ref::<HsError>(),
            Some(&HsError::ScanTerminated)
        );
    }

    #[test]
    fn test_vectored_scan() {
        let _ = pretty_env_logger::try_init();

        let db: VectoredDatabase = pattern! {"test"; CASELESS|SOM_LEFTMOST}.build().unwrap();
        let s = db.alloc().unwrap();

        let data = vec!["foo".as_bytes(), "test".as_bytes(), "bar".as_bytes()];

        db.scan::<_, _, &()>(data, &s, None, None).unwrap();

        let mut matches = vec![];

        fn callback<'a>(id: u32, from: u64, to: u64, flags: u32, matches: Option<Pin<&'a mut Vec<(u64, u64)>>>) -> u32 {
            assert_eq!(id, 0);
            assert_eq!(from, 3);
            assert_eq!(to, 7);
            assert_eq!(flags, 0);

            matches.unwrap().push((from, to));

            1
        };

        let data = vec!["foo".as_bytes(), "test".as_bytes(), "bar".as_bytes()];

        assert_eq!(
            db.scan(data, &s, Some(callback), Some(Pin::new(&mut matches)))
                .err()
                .unwrap()
                .downcast_ref::<HsError>(),
            Some(&HsError::ScanTerminated)
        );
    }

    #[test]
    fn test_streaming_scan() {
        let _ = pretty_env_logger::try_init();

        let db: StreamingDatabase = pattern! {"test"; CASELESS}.build().unwrap();

        let s = db.alloc().unwrap();
        let st = db.open_stream().unwrap();

        let data = vec!["foo", "test", "bar"];
        let mut matches = vec![];

        fn callback<'a>(id: u32, from: u64, to: u64, flags: u32, matches: Option<Pin<&'a mut Vec<(u64, u64)>>>) -> u32 {
            assert_eq!(id, 0);
            assert_eq!(from, 0);
            assert_eq!(to, 7);
            assert_eq!(flags, 0);

            matches.unwrap().push((from, to));

            0
        }

        for d in data {
            st.scan(d, &s, Some(callback), Some(Pin::new(&mut matches))).unwrap();
        }

        st.close(&s, Some(callback), Some(Pin::new(&mut matches))).unwrap();
    }

    #[test]
    fn test_scan_reader() {
        let mut buf = String::from_utf8(vec![b'x'; SCAN_BUF_SIZE - 2]).unwrap();

        buf.push_str("baaab");

        let db = pattern! { "a+"; SOM_LEFTMOST }.build::<Streaming>().unwrap();
        let s = db.alloc().unwrap();
        let mut cur = Cursor::new(buf.as_bytes());
        let mut matches = vec![];

        fn callback<'a>(
            _id: u32,
            from: u64,
            to: u64,
            _flags: u32,
            matches: Option<&RefCell<Pin<&mut Vec<(u64, u64)>>>>,
        ) -> u32 {
            matches.unwrap().borrow_mut().push((from, to));

            0
        }

        db.scan(
            &mut cur,
            &s,
            Some(callback),
            Some(&RefCell::new(Pin::new(&mut matches))),
        )
        .unwrap();

        assert_eq!(matches, vec![(4095, 4096), (4095, 4097), (4095, 4098)]);
    }
}
