use std::fmt;
use std::ptr;
use std::mem;
use std::os::raw::c_uint;

use raw::*;
use api::*;
use errors::Result;
use common::{BlockDatabase, StreamingDatabase, VectoredDatabase};

/// A large enough region of scratch space to support a given database.
///
pub struct RawScratch(RawScratchPtr);

impl fmt::Debug for RawScratch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawScratch({:p})", self.0)
    }
}

impl RawScratch {
    /// Allocate a "scratch" space for use by Hyperscan.
    ///
    /// This is required for runtime use, and one scratch space per thread,
    /// or concurrent caller, is required.
    ///
    fn alloc<D>(db: &D) -> Result<RawScratch>
    where
        D: AsPtr<Type = RawDatabaseType> + fmt::Debug,
    {
        let mut s: RawScratchPtr = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_alloc_scratch(db.as_ptr(), &mut s));
        }

        trace!(
            "allocated scratch at {:p} for {:?}",
            s,
            db,
        );

        Ok(RawScratch(s))
    }
}

impl Drop for RawScratch {
    fn drop(&mut self) {
        unsafe {
            assert_hs_error!(hs_free_scratch(self.0));

            trace!("freed scratch at {:p}", self.0);

            self.0 = ptr::null_mut();
        }
    }
}

impl Clone for RawScratch {
    fn clone(&self) -> Self {
        let mut s: RawScratchPtr = ptr::null_mut();

        unsafe {
            assert_hs_error!(hs_clone_scratch(self.0, &mut s));
        }

        trace!("cloned scratch from {:p} to {:p}", self.0, s);

        RawScratch(s)
    }
}

impl AsPtr for RawScratch {
    type Type = RawScratchType;

    fn as_ptr(&self) -> *const Self::Type {
        self.0
    }
}

impl AsMutPtr for RawScratch {
    fn as_mut_ptr(&mut self) -> *mut Self::Type {
        self.0
    }
}

impl Scratch for RawScratch {
    fn size(&self) -> Result<usize> {
        let mut size = 0;

        unsafe {
            check_hs_error!(hs_scratch_size(self.0, &mut size));
        }

        debug!("scratch {:p} size: {}", self.0, size);

        Ok(size)
    }


    fn realloc<D>(&mut self, db: &D) -> Result<&Self>
    where
        D: AsPtr<Type = RawDatabaseType> + fmt::Debug,
    {
        unsafe {
            check_hs_error!(hs_alloc_scratch(db.as_ptr(), &mut self.0));
        }

        trace!(
            "reallocated scratch {:p} for {:?}",
            self.0,
            db,
        );

        Ok(self)
    }
}

impl<D> ScratchAllocator<RawScratch> for D
where
    D: AsPtr<Type = RawDatabaseType> + fmt::Debug,
{
    fn alloc(&self) -> Result<RawScratch> {
        RawScratch::alloc(self)
    }

    fn realloc(&self, s: &mut RawScratch) -> Result<&Self> {
        s.realloc(self)?;

        Ok(self)
    }
}

impl<T: AsRef<[u8]>, S: Scratch> BlockScanner<T, S> for BlockDatabase {
    fn scan<D>(
        &self,
        data: T,
        flags: ScanFlags,
        scratch: &mut S,
        callback: Option<MatchEventCallback<D>>,
        context: Option<&D>,
    ) -> Result<&Self> {
        unsafe {
            let bytes = data.as_ref();

            check_hs_error!(hs_scan(
                self.as_ptr(),
                bytes.as_ptr() as *const i8,
                bytes.len() as u32,
                flags as u32,
                scratch.as_mut_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            ));

            trace!(
                "block scan {} bytes with {} database at {:p}",
                bytes.len(),
                self.name(),
                self.as_ptr(),
            )
        }

        Ok(self)
    }
}

impl<T: AsRef<[u8]>, S: Scratch> VectoredScanner<T, S> for VectoredDatabase {
    fn scan<D>(
        &self,
        data: &[T],
        flags: ScanFlags,
        scratch: &mut S,
        callback: Option<MatchEventCallback<D>>,
        context: Option<&D>,
    ) -> Result<&Self> {
        let mut ptrs = Vec::with_capacity(data.len());
        let mut lens = Vec::with_capacity(data.len());

        for d in data.iter() {
            let bytes = d.as_ref();
            ptrs.push(bytes.as_ptr() as *const i8);
            lens.push(bytes.len() as c_uint);
        }

        unsafe {
            check_hs_error!(hs_scan_vector(
                self.as_ptr(),
                ptrs.as_slice().as_ptr() as *const *const i8,
                lens.as_slice().as_ptr() as *const c_uint,
                data.len() as u32,
                flags as u32,
                scratch.as_mut_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            ));
        }

        trace!(
            "vectored scan {} bytes in {} parts with {} database at {:p}",
            lens.iter().fold(0, |sum, len| sum + len),
            lens.len(),
            self.name(),
            self.as_ptr(),
        );

        Ok(self)
    }
}

impl StreamingScanner<RawStream, RawScratch> for StreamingDatabase {
    fn open_stream(&self, flags: StreamFlags) -> Result<RawStream> {
        let mut id: RawStreamPtr = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_open_stream(self.as_ptr(), flags, &mut id));
        }

        trace!(
            "stream opened at {:p} for {} database at {:p}",
            id,
            self.name(),
            self.as_ptr(),
        );

        Ok(RawStream(id))
    }
}

/// A pattern matching state can be maintained across multiple blocks of target data
pub struct RawStream(RawStreamPtr);

impl fmt::Debug for RawStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawStream({:p})", self.0)
    }
}

impl AsPtr for RawStream {
    type Type = RawStreamType;

    fn as_ptr(&self) -> *const Self::Type {
        self.0
    }
}

impl Clone for RawStream {
    fn clone(&self) -> Self {
        let mut id: RawStreamPtr = ptr::null_mut();

        unsafe {
            assert_hs_error!(hs_copy_stream(&mut id, self.0));
        }

        debug!("stream cloned from {:p} to {:p}", self.0, id);

        RawStream(id)
    }
}

impl<S: Scratch> Stream<S> for RawStream {
    fn close<D>(&self, scratch: &mut S, callback: Option<MatchEventCallback<D>>, context: Option<&D>) -> Result<&Self> {
        unsafe {
            check_hs_error!(hs_close_stream(
                self.0,
                scratch.as_mut_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            ));
        }

        trace!("stream closed at {:p}", self.0);

        Ok(self)
    }

    fn reset<D>(
        &self,
        flags: StreamFlags,
        scratch: &mut S,
        callback: Option<MatchEventCallback<D>>,
        context: Option<&D>,
    ) -> Result<&Self> {
        unsafe {
            check_hs_error!(hs_reset_stream(
                self.0,
                flags,
                scratch.as_mut_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            ));
        }

        trace!("stream reset at {:p}", self.0);

        Ok(self)
    }
}

impl<T: AsRef<[u8]>, S: Scratch> BlockScanner<T, S> for RawStream {
    fn scan<D>(
        &self,
        data: T,
        flags: ScanFlags,
        scratch: &mut S,
        callback: Option<MatchEventCallback<D>>,
        context: Option<&D>,
    ) -> Result<&Self> {
        let bytes = data.as_ref();

        unsafe {
            check_hs_error!(hs_scan_stream(
                self.0,
                bytes.as_ptr() as *const i8,
                bytes.len() as u32,
                flags as u32,
                scratch.as_mut_ptr(),
                mem::transmute(callback),
                mem::transmute(context),
            ));
        }

        trace!(
            "stream scan {} bytes with stream at {:p}",
            bytes.len(),
            self.0
        );

        Ok(self)
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;
    use std::cell::RefCell;

    use super::super::*;
    use raw::AsPtr;

    const SCRATCH_SIZE: usize = 2000;

    #[test]
    fn test_scratch() {
        let db: BlockDatabase = pattern!{"test"}.build().unwrap();

        assert!(db.as_ptr() != ptr::null_mut());

        let s = db.alloc().unwrap();

        assert!(s.as_ptr() != ptr::null_mut());

        assert!(s.size().unwrap() > SCRATCH_SIZE);

        let mut s2 = s.clone();

        assert!(s2.as_ptr() != ptr::null_mut());

        assert!(s2.size().unwrap() > SCRATCH_SIZE);

        let db2: VectoredDatabase = pattern!{"foobar"}.build().unwrap();

        assert!(s2.realloc(&db2).unwrap().size().unwrap() > s.size().unwrap());
    }

    #[test]
    fn test_block_scan() {
        let db: BlockDatabase =
            pattern!{"test", flags => CompileFlags::HS_FLAG_CASELESS | CompileFlags::HS_FLAG_SOM_LEFTMOST}
                .build()
                .unwrap();
        let mut s = RawScratch::alloc(&db).unwrap();
        let matches = RefCell::new(Vec::new());

        extern "C" fn callback(_id: u32, from: u64, to: u64, _flags: u32, matches: &RefCell<Vec<(u64, u64)>>) -> u32 {
            (*matches.borrow_mut()).push((from, to));

            0
        };

        db.scan(
            "foo test bar".as_bytes(),
            0,
            &mut s,
            Some(callback),
            Some(&matches),
        ).unwrap();

        assert_eq!(matches.into_inner(), vec![(4, 8)]);
    }

    #[test]
    fn test_vectored_scan() {
        let db: VectoredDatabase =
            pattern!{"test", flags => CompileFlags::HS_FLAG_CASELESS | CompileFlags::HS_FLAG_SOM_LEFTMOST}
                .build()
                .unwrap();
        let mut s = RawScratch::alloc(&db).unwrap();
        let matches = RefCell::new(Vec::new());
        let data = vec!["foo", "test", "bar"];

        extern "C" fn callback(_id: u32, from: u64, to: u64, _flags: u32, matches: &RefCell<Vec<(u64, u64)>>) -> u32 {
            (*matches.borrow_mut()).push((from, to));

            0
        };

        db.scan(&data, 0, &mut s, Some(callback), Some(&matches))
            .unwrap();

        assert_eq!(matches.into_inner(), vec![(3, 7)]);
    }

    #[test]
    fn test_streaming_scan() {
        let db: StreamingDatabase =
            pattern!{"test", flags => CompileFlags::HS_FLAG_CASELESS | CompileFlags::HS_FLAG_SOM_LEFTMOST}
                .build()
                .unwrap();

        let mut s = RawScratch::alloc(&db).unwrap();
        let stream = db.open_stream(0).unwrap();
        let matches = RefCell::new(Vec::new());

        let data = vec!["foo", "test", "bar"];

        extern "C" fn callback(_id: u32, from: u64, to: u64, _flags: u32, matches: &RefCell<Vec<(u64, u64)>>) -> u32 {
            (*matches.borrow_mut()).push((from, to));

            0
        }

        for d in data {
            stream
                .scan(d, 0, &mut s, Some(callback), Some(&matches))
                .unwrap();
        }

        stream
            .close(&mut s, Some(callback), Some(&matches))
            .unwrap();

        assert_eq!(matches.into_inner(), vec![(3, 7)]);
    }
}
