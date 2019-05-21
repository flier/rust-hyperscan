use std::ffi::CStr;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use libc;

use api::*;
use cptr::CPtr;
use errors::Error;
use raw::*;

/// A compiled pattern database that can then be used to scan data.
pub struct RawDatabase<T: Type> {
    db: RawDatabasePtr,
    _marker: PhantomData<T>,
}

impl<T: Type> fmt::Debug for RawDatabase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawDatabase<{}>{{db: {:p}}}", T::name(), self.db)
    }
}

/// Block scan (non-streaming) database.
pub type BlockDatabase = RawDatabase<Block>;
/// Streaming database.
pub type StreamingDatabase = RawDatabase<Streaming>;
/// Vectored scanning database.
pub type VectoredDatabase = RawDatabase<Vectored>;

impl<T: Type> RawDatabase<T> {
    /// Constructs a compiled pattern database from a raw pointer.
    pub fn from_raw(db: RawDatabasePtr) -> RawDatabase<T> {
        trace!("construct {} database {:p}", T::name(), db);

        RawDatabase {
            db,
            _marker: PhantomData,
        }
    }

    /// Free a compiled pattern database.
    pub fn free(&mut self) -> Result<(), Error> {
        unsafe {
            check_hs_error!(hs_free_database(self.db));

            trace!("free {} database {:p}", T::name(), self.db);

            self.db = ptr::null_mut();

            Ok(())
        }
    }
}

impl<T: Type> Deref for RawDatabase<T> {
    type Target = RawDatabasePtr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl<T: Type> Database for RawDatabase<T> {
    fn database_mode(&self) -> u32 {
        T::mode()
    }

    fn database_name(&self) -> &'static str {
        T::name()
    }

    fn database_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_database_size(self.db, &mut size));
        }

        debug!("database size of {} database {:p}: {}", T::name(), self.db, size);

        Ok(size)
    }

    fn database_info(&self) -> Result<String, Error> {
        let mut p: *mut c_char = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_database_info(self.db, &mut p));

            let result = match CStr::from_ptr(p).to_str() {
                Ok(info) => Ok(info.to_string()),
                Err(_) => Err(Error::Invalid),
            };

            debug!("database info of {} database {:p}: {:?}", T::name(), self.db, result);

            libc::free(p as *mut libc::c_void);

            result
        }
    }
}

impl<T: Type> SerializableDatabase<RawDatabase<T>, RawSerializedDatabase> for RawDatabase<T> {
    fn serialize(&self) -> Result<RawSerializedDatabase, Error> {
        let mut bytes: *mut c_char = ptr::null_mut();
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_serialize_database(self.db, &mut bytes, &mut size));

            debug!("serialized {} database {:p} to {} bytes", T::name(), self.db, size);

            Ok(RawSerializedDatabase::from_raw_parts(bytes as *mut u8, size))
        }
    }

    fn deserialize(bytes: &[u8]) -> Result<RawDatabase<T>, Error> {
        let mut db: RawDatabasePtr = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_deserialize_database(
                bytes.as_ptr() as *const i8,
                bytes.len(),
                &mut db,
            ));

            debug!(
                "deserialized {} database to {:p} from {} bytes",
                T::name(),
                db,
                bytes.len()
            );
        }

        Ok(Self::from_raw(db))
    }

    fn deserialize_at(&self, bytes: &[u8]) -> Result<&RawDatabase<T>, Error> {
        unsafe {
            check_hs_error!(hs_deserialize_database_at(
                bytes.as_ptr() as *const i8,
                bytes.len(),
                self.db,
            ));

            debug!(
                "deserialized {} database at {:p} from {} bytes",
                T::name(),
                self.db,
                bytes.len()
            );

            Ok(self)
        }
    }
}

unsafe impl<T: Type> Send for RawDatabase<T> {}
unsafe impl<T: Type> Sync for RawDatabase<T> {}

impl<T: Type> Drop for RawDatabase<T> {
    #[inline]
    fn drop(&mut self) {
        self.free().unwrap()
    }
}

impl RawDatabase<Streaming> {
    pub fn stream_size(&self) -> Result<usize, Error> {
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_stream_size(self.db, &mut size));
        }

        Ok(size)
    }
}

pub struct RawSerializedDatabase {
    p: CPtr<u8>,
    len: usize,
}

impl fmt::Debug for RawSerializedDatabase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawSerializedDatabase{{p: {:p}, len: {}}}", self.p, self.len)
    }
}

impl RawSerializedDatabase {
    unsafe fn from_raw_parts(bytes: *mut u8, len: usize) -> RawSerializedDatabase {
        RawSerializedDatabase {
            p: CPtr::from_ptr(bytes),
            len,
        }
    }
}

impl SerializedDatabase for RawSerializedDatabase {
    fn len(&self) -> usize {
        self.len
    }

    fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(*self.p, self.len) }
    }
}

impl SerializedDatabase for [u8] {
    fn len(&self) -> usize {
        self.len()
    }

    fn as_slice(&self) -> &[u8] {
        self
    }
}

impl Deref for RawSerializedDatabase {
    type Target = *mut u8;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.p
    }
}

#[cfg(test)]
pub mod tests {
    extern crate env_logger;

    use std::ptr;

    use regex::Regex;

    use super::super::*;

    const DATABASE_SIZE: usize = 872;

    pub fn validate_database_info(info: &str) -> (Vec<u8>, Option<String>, Option<String>) {
        if let Some(captures) = Regex::new(r"^Version:\s(\d\.\d\.\d)\sFeatures:\s+(\w+)?\sMode:\s(\w+)$")
            .unwrap()
            .captures(info)
        {
            let version = captures
                .get(1)
                .unwrap()
                .as_str()
                .split('.')
                .flat_map(|s| s.parse())
                .collect();
            let features = captures.get(2).map(|m| m.as_str().to_owned());
            let mode = captures.get(3).map(|m| m.as_str().to_owned());

            (version, features, mode)
        } else {
            panic!("fail to parse database info: {}", info);
        }
    }

    pub fn validate_database_with_size<T: Database>(db: &T, size: usize) {
        assert!(db.database_size().unwrap() >= size);

        let db_info = db.database_info().unwrap();

        validate_database_info(&db_info);
    }

    pub fn validate_database<T: Database>(db: &T) {
        validate_database_with_size(db, DATABASE_SIZE);
    }

    pub fn validate_serialized_database<T: SerializedDatabase + ?Sized>(data: &T) {
        assert!(data.len() >= DATABASE_SIZE);
        assert!(data.database_size().unwrap() >= DATABASE_SIZE);

        let db_info = data.database_info().unwrap();

        validate_database_info(&db_info);
    }

    #[test]
    pub fn test_platform() {
        assert!(PlatformInfo::is_valid())
    }

    #[test]
    fn test_database() {
        let _ = env_logger::try_init();

        let db = BlockDatabase::compile("test", 0, &PlatformInfo::null()).unwrap();

        assert!(*db != ptr::null_mut());

        validate_database(&db);

        assert!(Regex::new(r"RawDatabase<Block>\{db: \w+\}")
            .unwrap()
            .is_match(&format!("{:?}", db)));
    }

    #[test]
    fn test_database_serialize() {
        let _ = env_logger::try_init();

        let db = StreamingDatabase::compile("test", 0, &PlatformInfo::null()).unwrap();

        let data = db.serialize().unwrap();

        assert!(*data != ptr::null_mut());

        validate_serialized_database(&data);
        validate_serialized_database(data.as_slice());

        assert!(Regex::new(r"RawSerializedDatabase\{p: \w+, len: \d+\}")
            .unwrap()
            .is_match(&format!("{:?}", data)));
    }

    #[test]
    fn test_database_deserialize() {
        let _ = env_logger::try_init();

        let db = VectoredDatabase::compile("test", 0, &PlatformInfo::null()).unwrap();

        let data = db.serialize().unwrap();

        let db = VectoredDatabase::deserialize(data.as_slice()).unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_database_deserialize_at() {
        let _ = env_logger::try_init();

        let db = BlockDatabase::compile("test", 0, &PlatformInfo::null()).unwrap();

        let data = db.serialize().unwrap();

        validate_database(db.deserialize_at(data.as_slice()).unwrap());
    }
}
