use std::fmt;
use std::mem::{self, MaybeUninit};
use std::ops::Range;
use std::slice;

use anyhow::Result;
use derive_more::{Deref, From};
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::chimera::{errors::AsResult, ffi, DatabaseRef};

foreign_type! {
    /// A large enough region of scratch space to support a given database.
    pub unsafe type Scratch {
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
    pub fn realloc_scratch(&mut self, s: Scratch) -> Result<Scratch> {
        let mut s = s.into_ptr();

        unsafe { ffi::ch_alloc_scratch(self.as_ptr(), &mut s).map(|_| Scratch::from_ptr(s)) }
    }
}

/// Callback return value used to tell the Chimera matcher what to do after processing this match.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Copy, Debug, From, PartialEq)]
pub enum Error {
    /// PCRE hits its match limit.
    MatchLimit = ffi::CH_ERROR_MATCHLIMIT,
    /// PCRE hits its recursion limit.
    RecursionLimit = ffi::CH_ERROR_RECURSIONLIMIT,
}

/// Structure representing a captured subexpression within a match.
#[repr(transparent)]
#[derive(Clone, Copy, From, Deref, PartialEq)]
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

unsafe extern "C" fn on_match_trampoline<F, E>(
    id: u32,
    from: u64,
    to: u64,
    flags: u32,
    size: u32,
    captured: *const ffi::ch_capture_t,
    ctx: *mut ::libc::c_void,
) -> ffi::ch_callback_t
where
    F: FnMut(u32, u64, u64, u32, Option<&[Capture]>) -> Matching,
    E: FnMut(Error, u32) -> Matching,
{
    let &mut (ref mut callback, _) = &mut *(ctx as *mut (&mut F, &mut E));

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

unsafe extern "C" fn on_error_trampoline<F, E>(
    error_type: ffi::ch_error_event_t,
    id: u32,
    _info: *mut ::libc::c_void,
    ctx: *mut ::libc::c_void,
) -> ffi::ch_callback_t
where
    F: FnMut(u32, u64, u64, u32, Option<&[Capture]>) -> Matching,
    E: FnMut(Error, u32) -> Matching,
{
    let &mut (_, ref mut callback) = &mut *(ctx as *mut (&mut F, &mut E));

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
    /// * `id`: The ID number of the expression that matched.
    /// * `from`: The offset of the first byte that matches the expression.
    /// * `to`: The offset after the last byte that matches the expression.
    /// * `flags`: This is provided for future use and is unused at present.
    /// * `captured`: An array of `Capture` structures that contain the start and end offsets of entire pattern match and each captured subexpression.
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
    pub fn scan<T, F, E>(
        &self,
        data: T,
        scratch: &ScratchRef,
        mut on_match_event: F,
        mut on_error_event: E,
    ) -> Result<()>
    where
        T: AsRef<[u8]>,
        F: FnMut(u32, u64, u64, u32, Option<&[Capture]>) -> Matching,
        E: FnMut(Error, u32) -> Matching,
    {
        let data = data.as_ref();

        let mut userdata = (&mut on_match_event, &mut on_error_event);

        unsafe {
            ffi::ch_scan(
                self.as_ptr(),
                data.as_ptr() as *const _,
                data.len() as _,
                0,
                scratch.as_ptr(),
                Some(on_match_trampoline::<F, E>),
                Some(on_error_trampoline::<F, E>),
                &mut userdata as *mut _ as *mut _,
            )
            .ok()
        }
    }
}
