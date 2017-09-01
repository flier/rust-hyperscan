pub type HsError = i32;

/// The engine completed normally.
pub const HS_SUCCESS: HsError = 0;

/// A parameter passed to this function was invalid.
pub const HS_INVALID: HsError = -1;

/// A memory allocation failed.
pub const HS_NOMEM: HsError = -2;

/// The engine was terminated by callback.
///
/// This return value indicates that the target buffer was partially scanned,
/// but that the callback function requested that scanning cease after a match
/// was located.
pub const HS_SCAN_TERMINATED: HsError = -3;

/// The pattern compiler failed, and the @ref hs_compile_error_t should be
/// inspected for more detail.
pub const HS_COMPILER_ERROR: HsError = -4;

/// The given database was built for a different version of Hyperscan.
pub const HS_DB_VERSION_ERROR: HsError = -5;

/// The given database was built for a different platform (i.e., CPU type).
pub const HS_DB_PLATFORM_ERROR: HsError = -6;

/// The given database was built for a different mode of operation. This error
/// is returned when streaming calls are used with a block or vectored database
/// and vice versa.
pub const HS_DB_MODE_ERROR: HsError = -7;

/// A parameter passed to this function was not correctly aligned.
pub const HS_BAD_ALIGN: HsError = -8;

/// The memory allocator (either malloc() or the allocator set with @ref
/// hs_set_allocator()) did not correctly return memory suitably aligned for the
/// largest representable data type on this platform.
pub const HS_BAD_ALLOC: HsError = -9;

/// The scratch region was already in use.
///
/// This error is returned when Hyperscan is able to detect that the scratch
/// region given is already in use by another Hyperscan API call.
///
/// A separate scratch region, allocated with @ref hs_alloc_scratch() or @ref
/// hs_clone_scratch(), is required for every concurrent caller of the Hyperscan
/// API.
///
/// For example, this error might be returned when @ref hs_scan() has been
/// called inside a callback delivered by a currently-executing @ref hs_scan()
/// call using the same scratch region.
///
/// Note: Not all concurrent uses of scratch regions may be detected. This error
/// is intended as a best-effort debugging tool, not a guarantee.
pub const HS_SCRATCH_IN_USE: HsError = -10;

/// Unsupported CPU architecture.
///
/// This error is returned when Hyperscan is able to detect that the current
/// system does not support the required instruction set.
///
/// At a minimum, Hyperscan requires Supplemental Streaming SIMD Extensions 3
/// (SSSE3).
pub const HS_ARCH_ERROR: HsError = -11;

bitflags! {
    pub struct CompileMode: u32 {
        #[doc="Compiler mode flag: Block scan (non-streaming) database."]
        const HS_MODE_BLOCK = 1;

        #[doc="Compiler mode flag: Alias for HS_MODE_BLOCK."]
        const HS_MODE_NOSTREAM = 1;

        #[doc="Compiler mode flag: Streaming database."]
        const HS_MODE_STREAM = 2;

        #[doc="Compiler mode flag: Vectored scanning database."]
        const HS_MODE_VECTORED = 4;

        #[doc="Compiler mode flag: use full precision to track start of match offsets in stream state."]
        const HS_MODE_SOM_HORIZON_LARGE = 1 << 24;

        #[doc="Compiler mode flag: use medium precision to track start of match offsets in stream state."]
        const HS_MODE_SOM_HORIZON_MEDIUM = 1 << 25;

        #[doc="Compiler mode flag: use limited precision to track start of match offsets in stream state."]
        const HS_MODE_SOM_HORIZON_SMALL = 1 << 26;
    }
}

bitflags! {
    pub struct CompileFlags: u32 {
        #[doc="Compile flag: Set case-insensitive matching."]
        const HS_FLAG_CASELESS = 1;

        #[doc="Compile flag: Matching a `.` will not exclude newlines."]
        const HS_FLAG_DOTALL = 2;

        #[doc="Compile flag: Set multi-line anchoring."]
        const HS_FLAG_MULTILINE = 4;

        #[doc="Compile flag: Set single-match only mode."]
        const HS_FLAG_SINGLEMATCH = 8;

        #[doc="Compile flag: Allow expressions that can match against empty buffers."]
        const HS_FLAG_ALLOWEMPTY = 16;

        #[doc="Compile flag: Enable UTF-8 mode for this expression."]
        const HS_FLAG_UTF8 = 32;

        #[doc="Compile flag: Enable Unicode property support for this expression."]
        const HS_FLAG_UCP = 64;

        #[doc="Compile flag: Enable prefiltering mode for this expression."]
        const HS_FLAG_PREFILTER = 128;

        #[doc="Compile flag: Enable leftmost start of match reporting."]
        const HS_FLAG_SOM_LEFTMOST = 256;
    }
}

/// CPU features flag - Intel(R) Advanced Vector Extensions 2 (Intel(R) AVX2)
///
/// Setting this flag indicates that the target platform supports AVX2
/// instructions.
pub const HS_CPU_FEATURES_AVX2: u32 = 1 << 2;

/// CPU features flag - Intel(R) Advanced Vector Extensions 512 (Intel(R) AVX512)
///
/// Setting this flag indicates that the target platform supports AVX512
/// instructions, specifically AVX-512BW. Using AVX512 implies the use of AVX2.
pub const HS_CPU_FEATURES_AVX512: u32 = 1 << 3;


/// Tuning Parameter - Generic
///
/// This indicates that the compiled database should not be tuned for any
/// particular target platform.
pub const HS_TUNE_FAMILY_GENERIC: u32 = 0;

/// Tuning Parameter - Intel(R) microarchitecture code name Sandy Bridge
///
/// This indicates that the compiled database should be tuned for the
/// Sandy Bridge microarchitecture.
pub const HS_TUNE_FAMILY_SNB: u32 = 1;

/// Tuning Parameter - Intel(R) microarchitecture code name Ivy Bridge
///
/// This indicates that the compiled database should be tuned for the
/// Ivy Bridge microarchitecture.
pub const HS_TUNE_FAMILY_IVB: u32 = 2;

/// Tuning Parameter - Intel(R) microarchitecture code name Haswell
///
/// This indicates that the compiled database should be tuned for the
/// Haswell microarchitecture.
pub const HS_TUNE_FAMILY_HSW: u32 = 3;

/// Tuning Parameter - Intel(R) microarchitecture code name Silvermont
///
/// This indicates that the compiled database should be tuned for the
/// Silvermont microarchitecture.
pub const HS_TUNE_FAMILY_SLM: u32 = 4;

/// Tuning Parameter - Intel(R) microarchitecture code name Broadwell
///
/// This indicates that the compiled database should be tuned for the
/// Broadwell microarchitecture.
pub const HS_TUNE_FAMILY_BDW: u32 = 5;

/// Tuning Parameter - Intel(R) microarchitecture code name Skylake
///
/// This indicates that the compiled database should be tuned for the
/// Skylake microarchitecture.
pub const HS_TUNE_FAMILY_SKL: u32 = 6;

/// Tuning Parameter - Intel(R) microarchitecture code name Skylake Server
///
/// This indicates that the compiled database should be tuned for the
/// Skylake Server microarchitecture.
pub const HS_TUNE_FAMILY_SKX: u32 = 7;

/// Tuning Parameter - Intel(R) microarchitecture code name Goldmont
///
/// This indicates that the compiled database should be tuned for the
/// Goldmont microarchitecture.
pub const HS_TUNE_FAMILY_GLM: u32 = 8;
