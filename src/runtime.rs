use std::ptr;
use std::mem;
use std::os::raw::c_void;
use std::ops::{Deref, Fn};

use raw::*;
use errors::Error;
use common::{BlockDatabase, VectoredDatabase, StreamingDatabase};

pub trait Scratch : Clone {
    fn size(&self) -> Result<usize, Error>;

    fn realloc(&mut self, db: *const hs_database_t) -> Result<&Self, Error>;
}

pub struct RawScratch {
    s: *mut hs_scratch_t,
}

impl RawScratch {
    pub fn alloc(db: *const hs_database_t) -> Result<RawScratch, Error> {
        let mut s: *mut hs_scratch_t = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_alloc_scratch(db, &mut s));
        }

        Result::Ok(RawScratch { s: s })
    }
}

impl Drop for RawScratch {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            assert_hs_error!(hs_free_scratch(self.s));
        }
    }
}

impl Clone for RawScratch {
    fn clone(&self) -> Self {
        let mut s: *mut hs_scratch_t = ptr::null_mut();

        unsafe {
            assert_hs_error!(hs_clone_scratch(self.s, &mut s));
        }

        RawScratch { s: s }
    }
}

impl Deref for RawScratch {
    type Target = *mut hs_scratch_t;

    #[inline]
    fn deref(&self) -> &*mut hs_scratch_t {
        &self.s
    }
}

impl Scratch for RawScratch {
    fn size(&self) -> Result<usize, Error> {
        let mut size: size_t = 0;

        unsafe {
            check_hs_error!(hs_scratch_size(self.s, &mut size));
        }

        Result::Ok(size as usize)
    }

    fn realloc(&mut self, db: *const hs_database_t) -> Result<&Self, Error> {
        unsafe {
            check_hs_error!(hs_alloc_scratch(db, &mut self.s));
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
    fn to_bytes(&self) -> &[u8];
}

impl<'a> Scannable for &'a [u8] {
    fn to_bytes(&self) -> &[u8] {
        &self
    }
}

impl<'a> Scannable for &'a str {
    fn to_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a> Scannable for &'a String {
    fn to_bytes(&self) -> &[u8] {
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

pub trait VectoredScanner<T: Scannable>{
    fn scan(&self,
            data: &Vec<T>,
            scratch: &RawScratch,
            handler: Option<&MatchEventCallback>)
            -> Result<&Self, Error>;
}

pub trait StreamingScanner {

}

impl<T: Scannable> BlockScanner<T> for BlockDatabase {
    fn scan(&self,
            data: T,
            scratch: &RawScratch,
            handler: Option<&MatchEventCallback>)
            -> Result<&Self, Error> {
        unsafe {
            let bytes = data.to_bytes();

            match handler {
                None => {
                    check_hs_error!(hs_scan(**self,
                                            bytes.as_ptr() as *const i8,
                                            bytes.len() as u32,
                                            0 as u32,
                                            **scratch,
                                            Option::None,
                                            ptr::null_mut()))
                }
                Some(callback) => {
                    check_hs_error!(hs_scan(**self,
                                            bytes.as_ptr() as *const i8,
                                            bytes.len() as u32,
                                            0 as u32,
                                            **scratch,
                                            Option::Some(match_event_callback),
                                            mem::transmute(&callback)))
                }
            }
        }

        Result::Ok(&self)
    }
}

impl<T: Scannable> VectoredScanner<T> for VectoredDatabase {
    fn scan(&self,
            data: &Vec<T>,
            scratch: &RawScratch,
            handler: Option<&MatchEventCallback>)
            -> Result<&Self, Error> {

        let mut ptrs = Vec::with_capacity(data.len());
        let mut lens = Vec::with_capacity(data.len());

        for d in data.iter() {
            let bytes = d.to_bytes();

            ptrs.push(bytes.as_ptr() as *const i8);
            lens.push(bytes.len() as uint32_t);
        }

        unsafe {
            match handler {
                None => {
                    check_hs_error!(hs_scan_vector(**self,
                                                   ptrs.as_slice().as_ptr() as *const *const i8,
                                                   lens.as_slice().as_ptr() as *const uint32_t,
                                                   data.len() as u32,
                                                   0 as u32,
                                                   **scratch,
                                                   Option::None,
                                                   ptr::null_mut()))
                }
                Some(callback) => {
                    check_hs_error!(hs_scan_vector(**self,
                                                   ptrs.as_slice().as_ptr() as *const *const i8,
                                                   lens.as_slice().as_ptr() as *const uint32_t,
                                                   data.len() as u32,
                                                   0 as u32,
                                                   **scratch,
                                                   Option::Some(match_event_callback),
                                                   mem::transmute(&callback)))
                }
            }
        }

        Result::Ok(&self)
    }
}

impl StreamingScanner for StreamingDatabase {}

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
        let db: BlockDatabase = pattern!{"test", flags => HS_FLAG_CASELESS| HS_FLAG_SOM_LEFTMOST}
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
        let db: VectoredDatabase =
            pattern!{"test", flags => HS_FLAG_CASELESS| HS_FLAG_SOM_LEFTMOST}
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
}
