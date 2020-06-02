use std::mem::{self, MaybeUninit};
use std::slice;

use anyhow::Result;
use derive_more::From;
use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use crate::{
    chimera::{errors::AsResult, ffi, DatabaseRef},
    Scannable,
};

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

impl Scratch {
    /// Allocate a "scratch" space for use by Chimera.
    ///
    /// This is required for runtime use, and one scratch space per thread,
    /// or concurrent caller, is required.
    ///
    unsafe fn alloc(db: &DatabaseRef) -> Result<Scratch> {
        let mut s = MaybeUninit::zeroed();
        ffi::ch_alloc_scratch(db.as_ptr(), s.as_mut_ptr()).map(|_| Scratch::from_ptr(s.assume_init()))
    }
}

impl DatabaseRef {
    /// Allocate a "scratch" space for use by Hyperscan.
    pub fn alloc_scratch(&self) -> Result<Scratch> {
        unsafe { Scratch::alloc(self) }
    }
}

/// Callback return value used to tell the Chimera matcher what to do after processing this match.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Matching {
    /// The matching should continue
    Continue = ffi::CH_CALLBACK_CONTINUE,
    /// The matching should cease
    Break = ffi::CH_CALLBACK_TERMINATE,
    /// Skip remaining matches for this ID and continue.
    Skip = ffi::CH_CALLBACK_SKIP_PATTERN,
}

impl Default for Matching {
    fn default() -> Self {
        Matching::Continue
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, From, PartialEq)]
pub enum Error {
    /// PCRE hits its match limit.
    MatchLimit = ffi::CH_ERROR_MATCHLIMIT,
    /// PCRE hits its recursion limit.
    RecursionLimit = ffi::CH_ERROR_RECURSIONLIMIT,
}

pub type Capture = ffi::ch_capture;

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
            Some(slice::from_raw_parts(captured, size as usize))
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
    /// ```rust
    /// use hyperscan::chimera::prelude::*;
    ///
    /// let pattern = pattern! {"test"; CASELESS};
    /// let db = pattern.build().unwrap();
    /// let scratch = db.alloc_scratch().unwrap();

    /// db.scan("some test data", &scratch, |id, from, to, _flags, captured| {
    ///     assert_eq!(id, 0);
    ///     assert_eq!(from, 5);
    ///     assert_eq!(to, 9);
    ///
    ///     println!("found pattern {} : {} @ [{}, {})", id, pattern.expression, from, to);
    ///
    ///     Matching::Continue
    /// }, |error_type, id| {
    ///     Matching::Skip
    /// }).unwrap();
    /// ```
    pub fn scan<T, F, E>(
        &self,
        data: T,
        scratch: &ScratchRef,
        mut on_match_event: F,
        mut on_error_event: E,
    ) -> Result<()>
    where
        T: Scannable,
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
