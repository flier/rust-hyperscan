use std::fmt;
use std::mem::{self, MaybeUninit};
use std::ops::Range;
use std::ptr;
use std::slice;

use derive_more::{Deref, From, Into};
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::{
    chimera::{error::AsResult, ffi, DatabaseRef},
    Result,
};

foreign_type! {
    /// A large enough region of scratch space to support a given database.
    pub unsafe type Scratch: Send {
        type CType = ffi::ch_scratch_t;

        fn drop = free_scratch;
        fn clone = clone_scratch;
    }
}

/// Free a scratch block previously allocated by `ch_alloc_scratch()` or `ch_clone_scratch()`.
unsafe fn free_scratch(s: *mut ffi::ch_scratch_t) {
    ffi::ch_free_scratch(s).expect("free scratch");
}

/// Allocate a scratch space that is a clone of an existing scratch space.
unsafe fn clone_scratch(s: *mut ffi::ch_scratch_t) -> *mut ffi::ch_scratch_t {
    let mut p = MaybeUninit::uninit();
    ffi::ch_clone_scratch(s, p.as_mut_ptr()).expect("clone scratch");
    p.assume_init()
}

impl ScratchRef {
    /// Provides the size of the given scratch space.
    pub fn size(&self) -> Result<usize> {
        let mut size = MaybeUninit::uninit();

        unsafe { ffi::ch_scratch_size(self.as_ptr(), size.as_mut_ptr()).map(|_| size.assume_init()) }
    }
}

impl DatabaseRef {
    /// Allocate a `scratch` space for use by Chimera.
    ///
    /// This is required for runtime use, and one scratch space per thread,
    /// or concurrent caller, is required.
    pub fn alloc_scratch(&self) -> Result<Scratch> {
        let mut s = MaybeUninit::zeroed();

        unsafe { ffi::ch_alloc_scratch(self.as_ptr(), s.as_mut_ptr()).map(|_| Scratch::from_ptr(s.assume_init())) }
    }

    /// Reallocate a `scratch` space for use by Chimera.
    pub fn realloc_scratch(&self, s: &mut Scratch) -> Result<&ScratchRef> {
        let mut p = s.as_ptr();

        unsafe {
            ffi::ch_alloc_scratch(self.as_ptr(), &mut p).map(|_| {
                s.0 = ptr::NonNull::new_unchecked(p);

                ScratchRef::from_ptr(p)
            })
        }
    }
}

/// Callback return value used to tell the Chimera matcher what to do after processing this match.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Matching {
    /// Continue matching.
    Continue = ffi::CH_CALLBACK_CONTINUE,
    /// Terminate matching.
    Terminate = ffi::CH_CALLBACK_TERMINATE,
    /// Skip remaining matches for this ID and continue.
    Skip = ffi::CH_CALLBACK_SKIP_PATTERN,
}

impl Default for Matching {
    fn default() -> Self {
        Matching::Continue
    }
}

/// The type of error event that occurred.
#[repr(u32)]
#[derive(Clone, Copy, Debug, From, PartialEq, Eq)]
pub enum Error {
    /// PCRE hits its match limit.
    MatchLimit = ffi::CH_ERROR_MATCHLIMIT,
    /// PCRE hits its recursion limit.
    RecursionLimit = ffi::CH_ERROR_RECURSIONLIMIT,
}

/// Structure representing a captured subexpression within a match.
#[repr(transparent)]
#[derive(Clone, Copy, From, Into, Deref, PartialEq, Eq)]
pub struct Capture(ffi::ch_capture);

impl fmt::Debug for Capture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Capture")
            .field("is_active", &self.is_active())
            .field("from", &self.from)
            .field("to", &self.to)
            .finish()
    }
}

impl From<Capture> for Range<usize> {
    fn from(capture: Capture) -> Self {
        capture.range()
    }
}

impl Capture {
    /// Indicating that a particular capture group is active
    pub fn is_active(&self) -> bool {
        self.flags == ffi::CH_CAPTURE_FLAG_ACTIVE
    }

    /// Returns the range of capture group
    pub fn range(&self) -> Range<usize> {
        self.from as usize..self.to as usize
    }
}

/// Definition of the match event callback function type.
///
/// A callback function matching the defined type must be provided by the
/// application calling the `DatabaseRef::scan`
///
/// This callback function will be invoked whenever a match is located in the
/// target data during the execution of a scan. The details of the match are
/// passed in as parameters to the callback function, and the callback function
/// should return a value indicating whether or not matching should continue on
/// the target data. If no callbacks are desired from a scan call, NULL may be
/// provided in order to suppress match production.
pub trait MatchEventHandler<'a> {
    /// Split the match event handler to callback and userdata.
    ///
    /// # Safety
    ///
    /// The returned function can only be called with the returned pointer, or a pointer to another C closure.
    unsafe fn split(&mut self) -> (ffi::ch_match_event_handler, *mut libc::c_void);
}

impl MatchEventHandler<'_> for () {
    unsafe fn split(&mut self) -> (ffi::ch_match_event_handler, *mut libc::c_void) {
        (None, ptr::null_mut())
    }
}

impl MatchEventHandler<'_> for Matching {
    unsafe fn split(&mut self) -> (ffi::ch_match_event_handler, *mut libc::c_void) {
        unsafe extern "C" fn trampoline(
            _id: u32,
            _from: u64,
            _to: u64,
            _flags: u32,
            _size: u32,
            _captured: *const ffi::ch_capture_t,
            ctx: *mut ::libc::c_void,
        ) -> ::libc::c_int {
            *(*(ctx as *mut (&mut Matching, *mut ()))).0 as _
        }

        (Some(trampoline), self as *mut _ as *mut _)
    }
}

impl<'a, F> MatchEventHandler<'a> for F
where
    F: FnMut(u32, u64, u64, u32, Option<&'a [Capture]>) -> Matching,
{
    unsafe fn split(&mut self) -> (ffi::ch_match_event_handler, *mut libc::c_void) {
        (Some(on_match_trampoline::<'a, F>), self as *mut _ as *mut _)
    }
}

unsafe extern "C" fn on_match_trampoline<'a, F>(
    id: u32,
    from: u64,
    to: u64,
    flags: u32,
    size: u32,
    captured: *const ffi::ch_capture_t,
    ctx: *mut ::libc::c_void,
) -> ffi::ch_callback_t
where
    F: FnMut(u32, u64, u64, u32, Option<&'a [Capture]>) -> Matching,
{
    let &mut (ref mut callback, _) = &mut *(ctx as *mut (&mut F, *mut ()));

    callback(
        id,
        from,
        to,
        flags,
        if captured.is_null() || size == 0 {
            None
        } else {
            Some(slice::from_raw_parts(captured as *const _, size as usize))
        },
    ) as i32
}

/// Definition of the Chimera error event callback function type.
///
/// A callback function matching the defined type may be provided by the
/// application calling the @ref ch_scan function. This callback function
/// will be invoked when an error event occurs during matching; this indicates
/// that some matches for a given expression may not be reported.
pub trait ErrorEventHandler {
    /// Split the match event handler to callback and userdata.
    ///
    /// # Safety
    ///
    /// The returned function can only be called with the returned pointer, or a pointer to another C closure.
    unsafe fn split(&mut self) -> (ffi::ch_error_event_handler, *mut libc::c_void);
}

impl ErrorEventHandler for () {
    unsafe fn split(&mut self) -> (ffi::ch_error_event_handler, *mut libc::c_void) {
        (None, ptr::null_mut())
    }
}
impl ErrorEventHandler for Matching {
    unsafe fn split(&mut self) -> (ffi::ch_error_event_handler, *mut libc::c_void) {
        unsafe extern "C" fn trampoline(
            _error_type: ffi::ch_error_event_t,
            _id: u32,
            _info: *mut ::libc::c_void,
            ctx: *mut ::libc::c_void,
        ) -> ffi::ch_callback_t {
            *(*(ctx as *mut (*mut (), &mut Matching))).1 as _
        }

        (Some(trampoline), self as *mut _ as *mut _)
    }
}

impl<F> ErrorEventHandler for F
where
    F: FnMut(Error, u32) -> Matching,
{
    unsafe fn split(&mut self) -> (ffi::ch_error_event_handler, *mut libc::c_void) {
        (Some(on_error_trampoline::<F>), self as *mut _ as *mut _)
    }
}

unsafe extern "C" fn on_error_trampoline<F>(
    error_type: ffi::ch_error_event_t,
    id: u32,
    _info: *mut ::libc::c_void,
    ctx: *mut ::libc::c_void,
) -> ffi::ch_callback_t
where
    F: FnMut(Error, u32) -> Matching,
{
    let &mut (_, ref mut callback) = &mut *(ctx as *mut (*mut (), &mut F));

    callback(mem::transmute(error_type), id) as i32
}

impl DatabaseRef {
    /// The block regular expression scanner.
    ///
    /// ## Handling Matches
    ///
    /// `scan` will call a user-supplied callback when a match is found.
    ///
    /// This closure has the following signature:
    ///
    /// ```rust,no_run
    /// # use hyperscan::chimera::{Capture, Matching};
    /// fn on_match_event(id: u32, from: u64, to: u64, flags: u32, captured: Option<&[Capture]>) -> Matching {
    ///     Matching::Continue
    /// }
    /// ```
    ///
    /// ### Parameters
    ///
    /// - `id`: The ID number of the expression that matched.
    /// - `from`: The offset of the first byte that matches the expression.
    /// - `to`: The offset after the last byte that matches the expression.
    /// - `flags`: This is provided for future use and is unused at present.
    /// - `captured`: An array of `Capture` structures that contain the start and end offsets of entire pattern match and each captured subexpression.
    ///
    /// ### Return
    ///
    /// The callback can return `Matching::Terminate` to stop matching.
    /// Otherwise, a return value of `Matching::Continue` will continue,
    /// with the current pattern if configured to produce multiple matches per pattern,
    /// while a return value of `Matching::Skip` will cease matching this pattern but continue matching the next pattern.
    ///
    /// ## Handling Runtime Errors
    ///
    /// `scan` will call a user-supplied callback when a runtime error occurs in libpcre.
    ///
    /// This closure has the following signature:
    ///
    /// ```rust,no_run
    /// # use hyperscan::chimera::{Error, Matching};
    /// fn on_error_event(error_type: Error, id: u32) -> Matching {
    ///     Matching::Continue
    /// }
    /// ```
    ///
    /// The `id` argument will be set to the identifier for the matching expression provided at compile time.
    ///
    /// The match callback has the capability to either halt scanning or continue scanning for the next pattern.
    ///
    /// ### Return
    ///
    /// The callback can return `Matching::Skip` to cease matching this pattern but continue matching the next pattern.
    /// Otherwise, we stop matching for all patterns with `Matching::Terminate`.
    pub fn scan<'a, T, F, E>(
        &self,
        data: T,
        scratch: &'a ScratchRef,
        mut on_match_event: F,
        mut on_error_event: E,
    ) -> Result<()>
    where
        T: AsRef<[u8]>,
        F: MatchEventHandler<'a>,
        E: ErrorEventHandler,
    {
        let data = data.as_ref();
        unsafe {
            let (on_match_callback, on_match_data) = on_match_event.split();
            let (on_error_callback, on_error_data) = on_error_event.split();

            let mut userdata = (on_match_data, on_error_data);

            ffi::ch_scan(
                self.as_ptr(),
                data.as_ptr() as *const _,
                data.len() as _,
                0,
                scratch.as_ptr(),
                on_match_callback,
                on_error_callback,
                &mut userdata as *mut _ as *mut _,
            )
            .ok()
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;

    use foreign_types::ForeignType;

    use crate::chimera::prelude::*;

    const SCRATCH_SIZE: usize = 2000;

    #[test]
    fn test_scratch() {
        let db: Database = "test".parse().unwrap();

        let s = db.alloc_scratch().unwrap();

        assert!(s.size().unwrap() > SCRATCH_SIZE);

        let mut s2 = s.clone();

        assert!(!ptr::eq(s.as_ptr(), s2.as_ptr()));

        assert!(s2.size().unwrap() > SCRATCH_SIZE);

        let db2: Database = "foobar".parse().unwrap();

        db2.realloc_scratch(&mut s2).unwrap();

        assert!(!ptr::eq(s.as_ptr(), s2.as_ptr()));
        assert!(s2.size().unwrap() >= s.size().unwrap());
    }
}
