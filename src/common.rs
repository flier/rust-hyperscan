use std::ptr;
use std::fmt;
use std::slice;
use std::ops::Deref;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::borrow::Cow;
use std::marker::PhantomData;

use libc;

use raw::*;
use api::*;
use constants::*;
use errors::Result;

/// Utility function for identifying this release version.
pub fn version<'a>() -> Cow<'a, str> {
    unsafe { CStr::from_ptr(hs_version()) }.to_string_lossy()
}

pub fn valid_platform() -> Result<()> {
    check_hs_error!(unsafe { hs_valid_platform() });

    Ok(())
}

/// Compile mode
pub trait DatabaseType {
    const MODE: CompileMode;
    const NAME: &'static str;
}

/// Block scan (non-streaming) database.
#[derive(Debug)]
pub enum Block {}

/// Streaming database.
#[derive(Debug)]
pub enum Streaming {}

/// Vectored scanning database.
#[derive(Debug)]
pub enum Vectored {}

impl DatabaseType for Block {
    const MODE: CompileMode = HS_MODE_BLOCK;
    const NAME: &'static str = "Block";
}

impl DatabaseType for Streaming {
    const MODE: CompileMode = HS_MODE_STREAM;
    const NAME: &'static str = "Streaming";
}
impl DatabaseType for Vectored {
    const MODE: CompileMode = HS_MODE_VECTORED;
    const NAME: &'static str = "Vectored";
}

/// A compiled pattern database that can then be used to scan data.
pub struct RawDatabase<T: DatabaseType> {
    db: RawDatabasePtr,
    _marker: PhantomData<T>,
}

impl<T: DatabaseType> fmt::Debug for RawDatabase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawDatabase<{}>{{db: {:p}}}", T::NAME, self.db)
    }
}

/// Block scan (non-streaming) database.
pub type BlockDatabase = RawDatabase<Block>;
/// Streaming database.
pub type StreamingDatabase = RawDatabase<Streaming>;
/// Vectored scanning database.
pub type VectoredDatabase = RawDatabase<Vectored>;

impl<T: DatabaseType> RawDatabase<T> {
    /// Constructs a compiled pattern database from a raw pointer.
    pub fn from_raw(db: RawDatabasePtr) -> RawDatabase<T> {
        trace!("construct {} database {:p}", T::NAME, db);

        RawDatabase {
            db: db,
            _marker: PhantomData,
        }
    }

    /// Free a compiled pattern database.
    pub fn free(&mut self) -> Result<()> {
        unsafe {
            check_hs_error!(hs_free_database(self.db));

            trace!("free {} database {:p}", T::NAME, self.db);

            self.db = ptr::null_mut();

            Ok(())
        }
    }
}

impl<T: DatabaseType> Deref for RawDatabase<T> {
    type Target = RawDatabasePtr;


    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl<T: DatabaseType> Database for RawDatabase<T> {
    fn database_mode(&self) -> CompileMode {
        T::MODE
    }

    fn database_name(&self) -> &'static str {
        T::NAME
    }

    fn database_size(&self) -> Result<usize> {
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_database_size(self.db, &mut size));
        }

        debug!(
            "database size of {} database {:p}: {}",
            T::NAME,
            self.db,
            size
        );

        Ok(size)
    }

    fn database_info(&self) -> Result<String> {
        let mut p: *mut c_char = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_database_info(self.db, &mut p));

            let result = match CStr::from_ptr(p).to_str() {
                Ok(info) => Ok(info.to_string()),
                Err(_) => bail!(hs_error!(Invalid)),
            };

            debug!(
                "database info of {} database {:p}: {:?}",
                T::NAME,
                self.db,
                result
            );

            libc::free(p as *mut libc::c_void);

            result
        }
    }
}

impl<'a, T: DatabaseType> SerializableDatabase<RawSerializedDatabase<'a>> for RawDatabase<T> {
    fn serialize(&self) -> Result<RawSerializedDatabase<'a>> {
        let mut bytes: *mut c_char = ptr::null_mut();
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_serialize_database(self.db, &mut bytes, &mut size));

            debug!(
                "serialized {} database {:p} to {} bytes",
                T::NAME,
                self.db,
                size
            );

            Ok(slice::from_raw_parts(bytes as *mut u8, size))
        }
    }

    fn deserialize(bytes: &[u8]) -> Result<RawDatabase<T>> {
        let mut db: RawDatabasePtr = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_deserialize_database(
                bytes.as_ptr() as *const i8,
                bytes.len(),
                &mut db,
            ));

            debug!(
                "deserialized {} database to {:p} from {} bytes",
                T::NAME,
                db,
                bytes.len()
            );
        }

        Ok(Self::from_raw(db))
    }

    fn deserialize_at(&self, bytes: &[u8]) -> Result<&RawDatabase<T>> {
        unsafe {
            check_hs_error!(hs_deserialize_database_at(
                bytes.as_ptr() as *const i8,
                bytes.len(),
                self.db,
            ));

            debug!(
                "deserialized {} database at {:p} from {} bytes",
                T::NAME,
                self.db,
                bytes.len()
            );

            Ok(self)
        }
    }
}

unsafe impl<T: DatabaseType> Send for RawDatabase<T> {}
unsafe impl<T: DatabaseType> Sync for RawDatabase<T> {}

impl<T: DatabaseType> Drop for RawDatabase<T> {
    fn drop(&mut self) {
        self.free().unwrap()
    }
}

impl RawDatabase<Streaming> {
    pub fn stream_size(&self) -> Result<usize> {
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_stream_size(self.db, &mut size));
        }

        Ok(size)
    }
}

pub type RawSerializedDatabase<'a> = &'a [u8];

impl<'a> SerializedDatabase for RawSerializedDatabase<'a> {}

#[cfg(test)]
pub mod tests {
    extern crate env_logger;

    use std::ptr;

    use regex::Regex;

    use super::super::*;

    const DATABASE_SIZE: usize = 872;

    pub fn validate_database_info(info: &str) -> (Vec<u8>, Option<String>, Option<String>) {
        if let Some(captures) = Regex::new(
            r"^Version:\s(\d\.\d\.\d)\sFeatures:\s+(\w+)?\sMode:\s(\w+)$",
        ).unwrap()
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
        assert_eq!(data.as_ref().len(), DATABASE_SIZE);
        assert_eq!(data.database_size().unwrap(), DATABASE_SIZE);

        let db_info = data.database_info().unwrap();

        validate_database_info(&db_info);
    }

    #[test]
    pub fn test_platform() {
        assert!(PlatformInfo::is_valid())
    }

    #[test]
    fn test_database() {
        let _ = env_logger::init();

        let db = BlockDatabase::compile("test", CompileFlags::default(), None).unwrap();

        assert!(*db != ptr::null_mut());

        validate_database(&db);
    }

    #[test]
    fn test_database_serialize() {
        let _ = env_logger::init();

        let db = StreamingDatabase::compile("test", CompileFlags::default(), None).unwrap();

        let data = db.serialize().unwrap();

        validate_serialized_database(&data);
    }

    #[test]
    fn test_database_deserialize() {
        let _ = env_logger::init();

        let db = VectoredDatabase::compile("test", CompileFlags::default(), None).unwrap();

        let data = db.serialize().unwrap();

        let db = VectoredDatabase::deserialize(data.as_ref()).unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_database_deserialize_at() {
        let _ = env_logger::init();

        let db = BlockDatabase::compile("test", CompileFlags::default(), None).unwrap();

        let data = db.serialize().unwrap();

        validate_database(db.deserialize_at(data.as_ref()).unwrap());
    }
}
