use std::ptr;
use std::fmt;
use std::ffi::CStr;
use std::ops::Deref;
use std::os::raw::c_char;
use std::slice;
use std::borrow::Cow;
use std::marker::PhantomData;

use libc;

use raw::*;
use api::{Database, DatabaseType, RawDatabasePtr, RawDatabaseType, SerializableDatabase, SerializedDatabase};
use constants::CompileMode;
use errors::Result;

/// Utility function for identifying this release version.
pub fn version<'a>() -> Cow<'a, str> {
    unsafe { CStr::from_ptr(hs_version()) }.to_string_lossy()
}

pub fn valid_platform() -> Result<()> {
    check_hs_error!(unsafe { hs_valid_platform() });

    Ok(())
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamMode {
    /// Streaming database use limited precision to track start of match offsets in stream state.
    ///
    /// This mode will use less stream state than `MediumStreaming` and
    /// will limit start of match accuracy to offsets within 2^16 bytes of the end of match offset reported.
    Small,
    /// Streaming database use medium precision to track start of match offsets in stream state.
    ///
    /// This mode will use less stream state than `LargeStreaming` and
    /// will limit start of match accuracy to offsets within 2^32 bytes of the end of match offset reported.
    Medium,
    /// Streaming database use full precision to track start of match offsets in stream state.
    ///
    /// This mode will use the most stream state per pattern,
    /// but will always return an accurate start of match offset regardless of how far back in the past it was found.
    Large,
}

impl Default for StreamMode {
    fn default() -> Self {
        StreamMode::Small
    }
}

impl From<StreamMode> for CompileMode {
    fn from(mode: StreamMode) -> Self {
        match mode {
            StreamMode::Small => CompileMode::HS_MODE_SOM_HORIZON_SMALL,
            StreamMode::Medium => CompileMode::HS_MODE_SOM_HORIZON_MEDIUM,
            StreamMode::Large => CompileMode::HS_MODE_SOM_HORIZON_LARGE,
        }
    }
}

impl DatabaseType for Block {
    fn mode() -> CompileMode {
        CompileMode::HS_MODE_BLOCK
    }

    fn name() -> &'static str {
        "Block"
    }
}

impl DatabaseType for Streaming {
    fn mode() -> CompileMode {
        CompileMode::HS_MODE_STREAM
    }
    fn name() -> &'static str {
        "Streaming"
    }
}

impl DatabaseType for Vectored {
    fn mode() -> CompileMode {
        CompileMode::HS_MODE_VECTORED
    }
    fn name() -> &'static str {
        "Vectored"
    }
}

/// A compiled pattern database that can then be used to scan data.
pub struct RawDatabase<T: DatabaseType> {
    db: RawDatabasePtr,
    _marker: PhantomData<T>,
}

impl<T: DatabaseType> fmt::Debug for RawDatabase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RawDatabase<{}>{{db: {:p}}}", T::name(), self.db)
    }
}

macro_rules! impl_database {
    ($name:ident, $mode:ty) => {
        impl ::std::ops::Deref for $name {
            type Target = RawDatabase<$mode>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl AsPtr for $name {
            type Type = RawDatabaseType;

            fn as_ptr(&self) -> *const Self::Type {
                self.0.as_ptr()
            }
        }
    };
}

/// Block scan (non-streaming) database.
#[derive(Debug)]
pub struct BlockDatabase(RawDatabase<Block>);

impl From<RawDatabase<Block>> for BlockDatabase {
    fn from(db: RawDatabase<Block>) -> Self {
        BlockDatabase(db)
    }
}

/// Streaming database.
#[derive(Debug)]
pub struct StreamingDatabase(RawDatabase<Streaming>, StreamMode);

impl From<RawDatabase<Streaming>> for StreamingDatabase {
    fn from(db: RawDatabase<Streaming>) -> Self {
        StreamingDatabase(db, StreamMode::default())
    }
}

/// Vectored scanning database.
#[derive(Debug)]
pub struct VectoredDatabase(RawDatabase<Vectored>);

impl From<RawDatabase<Vectored>> for VectoredDatabase {
    fn from(db: RawDatabase<Vectored>) -> Self {
        VectoredDatabase(db)
    }
}

impl_database!(BlockDatabase, Block);
impl_database!(StreamingDatabase, Streaming);
impl_database!(VectoredDatabase, Vectored);

impl<T: DatabaseType> RawDatabase<T> {
    /// Constructs a compiled pattern database from a raw pointer.
    pub fn from_raw(db: RawDatabasePtr) -> RawDatabase<T> {
        trace!("construct {} database {:p}", T::name(), db);

        RawDatabase {
            db: db,
            _marker: PhantomData,
        }
    }

    /// Free a compiled pattern database.
    pub fn free(&mut self) -> Result<()> {
        unsafe {
            check_hs_error!(hs_free_database(self.db));

            trace!("free {} database {:p}", T::name(), self.db);

            self.db = ptr::null_mut();

            Ok(())
        }
    }
}

impl<T: DatabaseType> AsPtr for RawDatabase<T> {
    type Type = RawDatabaseType;

    fn as_ptr(&self) -> *const Self::Type {
        self.db
    }
}

impl<D, T> Database for D
where
    D: Deref<Target = RawDatabase<T>>,
    T: DatabaseType,
{
    type DatabaseType = T;

    fn size(&self) -> Result<usize> {
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_database_size(self.db, &mut size));
        }

        debug!(
            "database size of {} database {:p}: {}",
            T::name(),
            self.db,
            size
        );

        Ok(size)
    }

    fn info(&self) -> Result<String> {
        let mut p: *mut c_char = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_database_info(self.db, &mut p));

            let result = CStr::from_ptr(p).to_str()?.to_owned();

            debug!(
                "database info of {} database {:p}: {:?}",
                T::name(),
                self.db,
                result
            );

            libc::free(p as *mut libc::c_void);

            Ok(result)
        }
    }
}

/// A pattern database was serialized to a stream of bytes.
impl<T: AsRef<[u8]>> SerializedDatabase for T {
    fn size(&self) -> Result<usize> {
        let buf = self.as_ref();
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_serialized_database_size(
                buf.as_ptr() as *const i8,
                buf.len(),
                &mut size,
            ));
        }

        Ok(size)
    }

    fn info(&self) -> Result<String> {
        let buf = self.as_ref();
        let mut p: *mut c_char = ptr::null_mut();

        unsafe {
            check_hs_error!(hs_serialized_database_info(
                buf.as_ptr() as *const i8,
                buf.len(),
                &mut p,
            ));

            let result = CStr::from_ptr(p).to_str()?.to_owned();

            libc::free(p as *mut libc::c_void);

            Ok(result)
        }
    }
}

pub struct RawSerializedDatabase {
    bytes: *mut u8,
    size: usize,
}

impl Drop for RawSerializedDatabase {
    fn drop(&mut self) {
        unsafe { libc::free(self.bytes as *mut libc::c_void) }
    }
}

impl AsRef<[u8]> for RawSerializedDatabase {
    fn as_ref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.bytes, self.size) }
    }
}

impl AsMut<[u8]> for RawSerializedDatabase {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.bytes, self.size) }
    }
}

impl RawSerializedDatabase {
    pub fn new(bytes: *mut u8, size: usize) -> Self {
        RawSerializedDatabase { bytes, size }
    }
}

impl<D, T> SerializableDatabase for D
where
    D: Deref<Target = RawDatabase<T>> + From<RawDatabase<T>>,
    T: DatabaseType,
{
    type Target = RawSerializedDatabase;

    fn serialize(&self) -> Result<Self::Target> {
        let mut bytes: *mut c_char = ptr::null_mut();
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_serialize_database(self.db, &mut bytes, &mut size));

            debug!(
                "serialized {} database {:p} to {} bytes",
                T::name(),
                self.db,
                size
            );
        }

        Ok(RawSerializedDatabase::new(bytes as *mut u8, size))
    }

    fn deserialize<B: AsRef<[u8]>>(buf: B) -> Result<Self> {
        let bytes = buf.as_ref();
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

        Ok(RawDatabase::from_raw(db).into())
    }

    fn deserialize_at<B: AsRef<[u8]>>(&self, buf: B) -> Result<&Self> {
        let bytes = buf.as_ref();

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

unsafe impl<T: DatabaseType> Send for RawDatabase<T> {}
unsafe impl<T: DatabaseType> Sync for RawDatabase<T> {}

impl<T: DatabaseType> Drop for RawDatabase<T> {
    fn drop(&mut self) {
        self.free().unwrap()
    }
}

impl StreamingDatabase {
    pub fn stream_mode(&self) -> StreamMode {
        self.1
    }

    pub fn set_stream_mode(&mut self, mode: StreamMode) {
        self.1 = mode
    }

    pub fn stream_size(&self) -> Result<usize> {
        let mut size: usize = 0;

        unsafe {
            check_hs_error!(hs_stream_size(self.db, &mut size));
        }

        Ok(size)
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;

    use re::Regex;

    use super::super::*;
    use raw::AsPtr;

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
        assert!(db.size().unwrap() >= size);

        let db_info = db.info().unwrap();

        validate_database_info(&db_info);
    }

    pub fn validate_database<T: Database>(db: &T) {
        validate_database_with_size(db, DATABASE_SIZE);
    }

    pub fn validate_serialized_database<T: SerializedDatabase>(data: &T) {
        assert_eq!(data.as_ref().len(), DATABASE_SIZE);
        assert_eq!(data.size().unwrap(), DATABASE_SIZE);

        let db_info = data.info().unwrap();

        validate_database_info(&db_info);
    }

    #[test]
    pub fn test_platform() {
        assert!(PlatformInfo::is_valid())
    }

    #[test]
    fn test_database() {
        let db = BlockDatabase::compile("test", CompileFlags::default(), None, None).unwrap();

        assert!(db.as_ptr() != ptr::null_mut());

        validate_database(&db);
    }

    #[test]
    fn test_database_serialize() {
        let db = StreamingDatabase::compile("test", CompileFlags::default(), None, None).unwrap();

        let data = db.serialize().unwrap();

        validate_serialized_database(&data);
    }

    #[test]
    fn test_database_deserialize() {
        let db = VectoredDatabase::compile("test", CompileFlags::default(), None, None).unwrap();

        let data = db.serialize().unwrap();

        let db = VectoredDatabase::deserialize(data.as_ref()).unwrap();

        validate_database(&db);
    }

    #[test]
    fn test_database_deserialize_at() {
        let db = BlockDatabase::compile("test", CompileFlags::default(), None, None).unwrap();

        let data = db.serialize().unwrap();

        validate_database(db.deserialize_at(data.as_ref()).unwrap());
    }
}
