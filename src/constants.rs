/**
 * The engine completed normally.
 */
pub const HS_SUCCESS: i32 = 0;

/**
 * A parameter passed to this function was invalid.
 */
pub const HS_INVALID: i32 = -1;

/**
 * A memory allocation failed.
 */
pub const HS_NOMEM: i32 = -2;

/**
 * The engine was terminated by callback.
 *
 * This return value indicates that the target buffer was partially scanned,
 * but that the callback function requested that scanning cease after a match
 * was located.
 */
pub const HS_SCAN_TERMINATED: i32 = -3;

/**
 * The pattern compiler failed, and the @ref hs_compile_error_t should be
 * inspected for more detail.
 */
pub const HS_COMPILER_ERROR: i32 = -4;

/**
 * The given database was built for a different version of Hyperscan.
 */
pub const HS_DB_VERSION_ERROR: i32 = -5;

/**
 * The given database was built for a different platform (i.e., CPU type).
 */
pub const HS_DB_PLATFORM_ERROR: i32 = -6;

/**
 * The given database was built for a different mode of operation. This error
 * is returned when streaming calls are used with a block or vectored database
 * and vice versa.
 */
pub const HS_DB_MODE_ERROR: i32 = -7;

/**
 * A parameter passed to this function was not correctly aligned.
 */
pub const HS_BAD_ALIGN: i32 = -8;

/**
 * The memory allocator (either malloc() or the allocator set with @ref
 * hs_set_allocator()) did not correctly return memory suitably aligned for the
 * largest representable data type on this platform.
 */
pub const HS_BAD_ALLOC: i32 = -9;

/**
 * Compiler mode flag: Block scan (non-streaming) database.
 */
pub const HS_MODE_BLOCK: u32 = 1;

/**
 * Compiler mode flag: Streaming database.
 */
pub const HS_MODE_STREAM: u32 = 2;

/**
 * Compiler mode flag: Vectored scanning database.
 */
pub const HS_MODE_VECTORED: u32 = 4;



/**
 * Compile flag: Set case-insensitive matching.
 *
 * This flag sets the expression to be matched case-insensitively by default.
 * The expression may still use PCRE tokens (notably `(?i)` and
 * `(?-i)`) to switch case-insensitive matching on and off.
 */
pub const HS_FLAG_CASELESS: u32 = 1;

/**
 * Compile flag: Matching a `.` will not exclude newlines.
 *
 * This flag sets any instances of the `.` token to match newline characters as
 * well as all other characters. The PCRE specification states that the `.`
 * token does not match newline characters by default, so without this flag the
 * `.` token will not cross line boundaries.
 */
pub const HS_FLAG_DOTALL: u32 = 2;

/**
 * Compile flag: Set multi-line anchoring.
 *
 * This flag instructs the expression to make the `^` and `$` tokens match
 * newline characters as well as the start and end of the stream. If this flag
 * is not specified, the `^` token will only ever match at the start of a
 * stream, and the `$` token will only ever match at the end of a stream within
 * the guidelines of the PCRE specification.
 */
pub const HS_FLAG_MULTILINE: u32 = 4;

/**
 * Compile flag: Set single-match only mode.
 *
 * This flag sets the expression's match ID to match at most once. In streaming
 * mode, this means that the expression will return only a single match over
 * the lifetime of the stream, rather than reporting every match as per
 * standard Hyperscan semantics. In block mode or vectored mode, only the first
 * match for each invocation of @ref hs_scan() or @ref hs_scan_vector() will be
 * returned.
 *
 * If multiple expressions in the database share the same match ID, then they
 * either must all specify @ref HS_FLAG_SINGLEMATCH or none of them specify
 * @ref HS_FLAG_SINGLEMATCH. If a group of expressions sharing a match ID
 * specify the flag, then at most one match with the match ID will be generated
 * per stream.
 *
 * Note: The use of this flag in combination with @ref HS_FLAG_SOM_LEFTMOST
 * is not currently supported.
 */
pub const HS_FLAG_SINGLEMATCH: u32 = 8;

/**
 * Compile flag: Allow expressions that can match against empty buffers.
 *
 * This flag instructs the compiler to allow expressions that can match against
 * empty buffers, such as `.?`, `.*`, `(a|)`. Since Hyperscan can return every
 * possible match for an expression, such expressions generally execute very
 * slowly; the default behaviour is to return an error when an attempt to
 * compile one is made. Using this flag will force the compiler to allow such
 * an expression.
 */
pub const HS_FLAG_ALLOWEMPTY: u32 = 16;

/**
 * Compile flag: Enable UTF-8 mode for this expression.
 *
 * This flag instructs Hyperscan to treat the pattern as a sequence of UTF-8
 * characters. The results of scanning invalid UTF-8 sequences with a Hyperscan
 * library that has been compiled with one or more patterns using this flag are
 * undefined.
 */
pub const HS_FLAG_UTF8: u32 = 32;

/**
 * Compile flag: Enable Unicode property support for this expression.
 *
 * This flag instructs Hyperscan to use Unicode properties, rather than the
 * default ASCII interpretations, for character mnemonics like `\w` and `\s` as
 * well as the POSIX character classes. It is only meaningful in conjunction
 * with @ref HS_FLAG_UTF8.
 */
pub const HS_FLAG_UCP: u32 = 64;

/**
 * Compile flag: Enable prefiltering mode for this expression.
 *
 * This flag instructs Hyperscan to compile an "approximate" version of this
 * pattern for use in a prefiltering application, even if Hyperscan does not
 * support the pattern in normal operation.
 *
 * The set of matches returned when this flag is used is guaranteed to be a
 * superset of the matches specified by the non-prefiltering expression.
 *
 * If the pattern contains pattern constructs not supported by Hyperscan (such
 * as zero-width assertions, back-references or conditional references) these
 * constructs will be replaced internally with broader constructs that may
 * match more often.
 *
 * Furthermore, in prefiltering mode Hyperscan may simplify a pattern that
 * would otherwise return a "Pattern too large" error at compile time, or for
 * performance reasons (subject to the matching guarantee above).
 *
 * It is generally expected that the application will subsequently confirm
 * prefilter matches with another regular expression matcher that can provide
 * exact matches for the pattern.
 *
 * Note: The use of this flag in combination with @ref HS_FLAG_SOM_LEFTMOST
 * is not currently supported.
 */
pub const HS_FLAG_PREFILTER: u32 = 128;

/**
 * Compile flag: Enable leftmost start of match reporting.
 *
 * This flag instructs Hyperscan to report the leftmost possible start of match
 * offset when a match is reported for this expression. (By default, no start
 * of match is returned.)
 *
 * Enabling this behaviour may reduce performance and increase stream state
 * requirements in streaming mode.
 */
pub const HS_FLAG_SOM_LEFTMOST: u32 = 256;
