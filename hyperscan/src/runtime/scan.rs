use core::pin::Pin;
use core::ptr::null_mut;
use std::io::Read;

use failure::Error;
use foreign_types::ForeignTypeRef;
use libc::{c_int, c_uint, c_ulonglong, c_void};

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
pub type MatchEventCallback<D> = fn(id: u32, from: u64, to: u64, data: Option<D>) -> Matching;

/// Indicating whether or not matching should continue on the target data.
pub enum Matching {
    /// The matching should continue
    Continue,
    /// The matching should cease
    Break,
}

pub struct MatchContext<D: Clone> {
    pub callback: MatchEventCallback<D>,
    pub data: Option<D>,
}

impl<D> MatchContext<D>
where
    D: Clone + Unpin,
{
    pub fn new(callback: MatchEventCallback<D>, data: Option<D>) -> Pin<Box<Self>> {
        Pin::new(Box::new(Self { callback, data }))
    }

    pub unsafe extern "C" fn stub(
        id: c_uint,
        from: c_ulonglong,
        to: c_ulonglong,
        _flags: c_uint,
        context: *mut c_void,
    ) -> c_int {
        let ctxt = (context as *mut Pin<Box<MatchContext<D>>>).as_mut().unwrap();

        match (ctxt.callback)(id, from, to, ctxt.data.clone()) {
            Matching::Continue => 0,
            Matching::Break => 1,
        }
    }
}

impl DatabaseRef<Block> {
    /// pattern matching takes place for block-mode pattern databases.
    pub fn scan<T, D>(
        &self,
        data: T,
        scratch: &ScratchRef,
        callback: Option<MatchEventCallback<D>>,
        context: Option<D>,
    ) -> Result<(), Error>
    where
        T: Scannable,
        D: Clone + Unpin,
    {
        let data = data.as_ref();
        let mut ctxt = callback.map(|callback| MatchContext::new(callback, context));

        unsafe {
            ffi::hs_scan(
                self.as_ptr(),
                data.as_ptr() as *const i8,
                data.len() as u32,
                0,
                scratch.as_ptr(),
                if ctxt.is_some() {
                    Some(MatchContext::<D>::stub)
                } else {
                    None
                },
                ctxt.as_mut().map_or_else(null_mut, |ctxt| ctxt as *mut _ as *mut _),
            )
            .ok()
        }
    }
}

impl DatabaseRef<Vectored> {
    /// pattern matching takes place for vectoring-mode pattern databases.
    pub fn scan<I, T, D>(
        &self,
        data: I,
        scratch: &ScratchRef,
        callback: Option<MatchEventCallback<D>>,
        context: Option<D>,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = T>,
        T: Scannable,
        D: Clone + Unpin,
    {
        let (ptrs, lens): (Vec<_>, Vec<_>) = data
            .into_iter()
            .map(|buf| {
                let buf = buf.as_ref();

                (buf.as_ptr() as *const i8, buf.len() as c_uint)
            })
            .unzip();
        let mut ctxt = callback.map(|callback| MatchContext::new(callback, context));

        unsafe {
            ffi::hs_scan_vector(
                self.as_ptr(),
                ptrs.as_slice().as_ptr() as *const *const i8,
                lens.as_slice().as_ptr() as *const _,
                ptrs.len() as u32,
                0,
                scratch.as_ptr(),
                if ctxt.is_some() {
                    Some(MatchContext::<D>::stub)
                } else {
                    None
                },
                ctxt.as_mut().map_or_else(null_mut, |ctxt| ctxt as *mut _ as *mut _),
            )
            .ok()
        }
    }
}

const SCAN_BUF_SIZE: usize = 4096;

impl DatabaseRef<Streaming> {
    /// pattern matching takes place for stream-mode pattern databases.
    pub fn scan<R, D>(
        &self,
        reader: &mut R,
        scratch: &ScratchRef,
        callback: Option<MatchEventCallback<D>>,
        context: Option<D>,
    ) -> Result<(), Error>
    where
        R: Read,
        D: Clone + Unpin,
    {
        let stream = self.open_stream()?;
        let mut buf = [0; SCAN_BUF_SIZE];

        while let Ok(len) = reader.read(&mut buf[..]) {
            if len == 0 {
                break;
            }

            stream.scan(&buf[..len], scratch, callback, context.clone())?;
        }

        stream.close(scratch, callback, context)
    }
}

impl StreamRef {
    /// pattern matching takes place for stream-mode pattern databases.
    pub fn scan<T, D>(
        &self,
        data: T,
        scratch: &ScratchRef,
        callback: Option<MatchEventCallback<D>>,
        context: Option<D>,
    ) -> Result<(), Error>
    where
        T: Scannable,
        D: Clone + Unpin,
    {
        let data = data.as_ref();
        let mut ctxt = callback.map(|callback| MatchContext::new(callback, context));

        unsafe {
            ffi::hs_scan_stream(
                self.as_ptr(),
                data.as_ptr() as *const i8,
                data.len() as u32,
                0,
                scratch.as_ptr(),
                if ctxt.is_some() {
                    Some(MatchContext::<D>::stub)
                } else {
                    None
                },
                ctxt.as_mut().map_or_else(null_mut, |ctxt| ctxt as *mut _ as *mut _),
            )
            .ok()
        }
    }
}

#[cfg(test)]
pub mod tests {
    use core::cell::RefCell;
    use std::io::Cursor;

    use crate::errors::HsError;
    use crate::prelude::*;

    use super::*;

    #[test]
    fn test_block_scan() {
        let _ = pretty_env_logger::try_init();

        let db: BlockDatabase = pattern! {"test"; CASELESS | SOM_LEFTMOST}.build().unwrap();
        let s = db.alloc().unwrap();

        db.scan::<_, &()>("foo test bar", &s, None, None).unwrap();

        fn callback<T>(id: u32, from: u64, to: u64, _: T) -> Matching {
            assert_eq!(id, 0);
            assert_eq!(from, 4);
            assert_eq!(to, 8);

            Matching::Break
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

        let data = vec!["foo", "test", "bar"];

        db.scan::<_, _, &()>(data, &s, None, None).unwrap();

        let matches = RefCell::new(vec![]);

        fn callback(id: u32, from: u64, to: u64, matches: Option<&RefCell<Vec<(u64, u64)>>>) -> Matching {
            assert_eq!(id, 0);
            assert_eq!(from, 3);
            assert_eq!(to, 7);

            matches.unwrap().borrow_mut().push((from, to));

            Matching::Break
        };

        let data = vec!["foo", "test", "bar"];

        assert_eq!(
            db.scan(data, &s, Some(callback), Some(&matches))
                .err()
                .unwrap()
                .downcast_ref::<HsError>(),
            Some(&HsError::ScanTerminated)
        );
    }

    #[test]
    fn test_streaming_scan() {
        let _ = pretty_env_logger::try_init();

        let db: StreamingDatabase = pattern! {"test"; SOM_LEFTMOST}.build().unwrap();

        let s = db.alloc().unwrap();
        let st = db.open_stream().unwrap();

        let data = vec!["foo t", "es", "t bar"];
        let matches = RefCell::new(vec![]);

        fn callback(_id: u32, from: u64, to: u64, matches: Option<&RefCell<Vec<(u64, u64)>>>) -> Matching {
            matches.unwrap().borrow_mut().push((from, to));

            Matching::Continue
        }

        for d in data {
            st.scan(d, &s, Some(callback), Some(&matches)).unwrap();
        }

        st.close(&s, Some(callback), Some(&matches)).unwrap();

        assert_eq!(matches.borrow().as_slice(), &[(4, 8)]);
    }

    #[test]
    fn test_scan_reader() {
        let _ = pretty_env_logger::try_init();

        let mut buf = String::from_utf8(vec![b'x'; SCAN_BUF_SIZE - 2]).unwrap();

        buf.push_str("baaab");

        let db = pattern! { "a+"; SOM_LEFTMOST }.build::<Streaming>().unwrap();
        let s = db.alloc().unwrap();
        let mut cur = Cursor::new(buf.as_bytes());
        let matches = RefCell::new(vec![]);

        fn callback(_id: u32, from: u64, to: u64, matches: Option<&RefCell<Vec<(u64, u64)>>>) -> Matching {
            matches.unwrap().borrow_mut().push((from, to));

            Matching::Continue
        }

        db.scan(&mut cur, &s, Some(callback), Some(&matches)).unwrap();

        assert_eq!(matches.borrow().as_slice(), &[(4095, 4096), (4095, 4097), (4095, 4098)]);
    }
}
