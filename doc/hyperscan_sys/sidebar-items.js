initSidebarItems({"constant":[["HS_ARCH_ERROR",""],["HS_BAD_ALIGN",""],["HS_BAD_ALLOC",""],["HS_COMPILER_ERROR",""],["HS_CPU_FEATURES_AVX2",""],["HS_CPU_FEATURES_AVX512",""],["HS_CPU_FEATURES_AVX512VBMI",""],["HS_DB_MODE_ERROR",""],["HS_DB_PLATFORM_ERROR",""],["HS_DB_VERSION_ERROR",""],["HS_EXT_FLAG_EDIT_DISTANCE",""],["HS_EXT_FLAG_HAMMING_DISTANCE",""],["HS_EXT_FLAG_MAX_OFFSET",""],["HS_EXT_FLAG_MIN_LENGTH",""],["HS_EXT_FLAG_MIN_OFFSET",""],["HS_FLAG_ALLOWEMPTY",""],["HS_FLAG_CASELESS",""],["HS_FLAG_COMBINATION",""],["HS_FLAG_DOTALL",""],["HS_FLAG_MULTILINE",""],["HS_FLAG_PREFILTER",""],["HS_FLAG_QUIET",""],["HS_FLAG_SINGLEMATCH",""],["HS_FLAG_SOM_LEFTMOST",""],["HS_FLAG_UCP",""],["HS_FLAG_UTF8",""],["HS_INSUFFICIENT_SPACE",""],["HS_INVALID",""],["HS_MAJOR",""],["HS_MINOR",""],["HS_MODE_BLOCK",""],["HS_MODE_NOSTREAM",""],["HS_MODE_SOM_HORIZON_LARGE",""],["HS_MODE_SOM_HORIZON_MEDIUM",""],["HS_MODE_SOM_HORIZON_SMALL",""],["HS_MODE_STREAM",""],["HS_MODE_VECTORED",""],["HS_NOMEM",""],["HS_OFFSET_PAST_HORIZON",""],["HS_PATCH",""],["HS_SCAN_TERMINATED",""],["HS_SCRATCH_IN_USE",""],["HS_SUCCESS",""],["HS_TUNE_FAMILY_BDW",""],["HS_TUNE_FAMILY_GENERIC",""],["HS_TUNE_FAMILY_GLM",""],["HS_TUNE_FAMILY_HSW",""],["HS_TUNE_FAMILY_ICL",""],["HS_TUNE_FAMILY_ICX",""],["HS_TUNE_FAMILY_IVB",""],["HS_TUNE_FAMILY_SKL",""],["HS_TUNE_FAMILY_SKX",""],["HS_TUNE_FAMILY_SLM",""],["HS_TUNE_FAMILY_SNB",""],["HS_UNKNOWN_ERROR",""]],"fn":[["hs_alloc_scratch","Allocate a “scratch” space for use by Hyperscan."],["hs_clone_scratch","Allocate a scratch space that is a clone of an existing scratch space."],["hs_close_stream","Close a stream."],["hs_compile","The basic regular expression compiler."],["hs_compile_ext_multi","The multiple regular expression compiler with extended parameter support."],["hs_compile_lit","The basic pure literal expression compiler."],["hs_compile_lit_multi","The multiple pure literal expression compiler."],["hs_compile_multi","The multiple regular expression compiler."],["hs_compress_stream","Creates a compressed representation of the provided stream in the buffer provided. This compressed representation can be converted back into a stream state by using @ref hs_expand_stream() or @ref hs_reset_and_expand_stream(). The size of the compressed representation will be placed into @p used_space."],["hs_copy_stream","Duplicate the given stream. The new stream will have the same state as the original including the current stream offset."],["hs_database_info","Utility function providing information about a database."],["hs_database_size","Provides the size of the given database in bytes."],["hs_deserialize_database","Reconstruct a pattern database from a stream of bytes previously generated by @ref hs_serialize_database()."],["hs_deserialize_database_at","Reconstruct a pattern database from a stream of bytes previously generated by @ref hs_serialize_database() at a given memory location."],["hs_expand_stream","Decompresses a compressed representation created by @ref hs_compress_stream() into a new stream."],["hs_expression_ext_info","Utility function providing information about a regular expression, with extended parameter support. The information provided in @ref hs_expr_info_t includes the minimum and maximum width of a pattern match."],["hs_expression_info","Utility function providing information about a regular expression. The information provided in @ref hs_expr_info_t includes the minimum and maximum width of a pattern match."],["hs_free_compile_error","Free an error structure generated by @ref hs_compile(), @ref hs_compile_multi() or @ref hs_compile_ext_multi()."],["hs_free_database","Free a compiled pattern database."],["hs_free_scratch","Free a scratch block previously allocated by @ref hs_alloc_scratch() or @ref hs_clone_scratch()."],["hs_open_stream","Open and initialise a stream."],["hs_populate_platform","Populates the platform information based on the current host."],["hs_reset_and_copy_stream","Duplicate the given ‘from’ stream state onto the ‘to’ stream. The ‘to’ stream will first be reset (reporting any EOD matches if a non-NULL @p onEvent callback handler is provided)."],["hs_reset_and_expand_stream","Decompresses a compressed representation created by @ref hs_compress_stream() on top of the ‘to’ stream. The ‘to’ stream will first be reset (reporting any EOD matches if a non-NULL @p onEvent callback handler is provided)."],["hs_reset_stream","Reset a stream to an initial state."],["hs_scan","The block (non-streaming) regular expression scanner."],["hs_scan_stream","Write data to be scanned to the opened stream."],["hs_scan_vector","The vectored regular expression scanner."],["hs_scratch_size","Provides the size of the given scratch space."],["hs_serialize_database","Serialize a pattern database to a stream of bytes."],["hs_serialized_database_info","Utility function providing information about a serialized database."],["hs_serialized_database_size","Utility function for reporting the size that would be required by a database if it were deserialized."],["hs_set_allocator","Set the allocate and free functions used by Hyperscan for allocating memory at runtime for stream state, scratch space, database bytecode, and various other data structure returned by the Hyperscan API."],["hs_set_database_allocator","Set the allocate and free functions used by Hyperscan for allocating memory for database bytecode produced by the compile calls (@ref hs_compile(), @ref hs_compile_multi(), @ref hs_compile_ext_multi()) and by database deserialization (@ref hs_deserialize_database())."],["hs_set_misc_allocator","Set the allocate and free functions used by Hyperscan for allocating memory for items returned by the Hyperscan API such as @ref hs_compile_error_t, @ref hs_expr_info_t and serialized databases."],["hs_set_scratch_allocator","Set the allocate and free functions used by Hyperscan for allocating memory for scratch space by @ref hs_alloc_scratch() and @ref hs_clone_scratch()."],["hs_set_stream_allocator","Set the allocate and free functions used by Hyperscan for allocating memory for stream state by @ref hs_open_stream()."],["hs_stream_size","Provides the size of the stream state allocated by a single stream opened against the given database."],["hs_valid_platform","Utility function to test the current system architecture."],["hs_version","Utility function for identifying this release version."]],"mod":[["chimera","Chimera is a software regular expression matching engine that is a hybrid of Hyperscan and PCRE."]],"struct":[["hs_compile_error","A type containing error details that is returned by the compile calls (@ref hs_compile(), @ref hs_compile_multi() and @ref hs_compile_ext_multi()) on failure. The caller may inspect the values returned in this type to determine the cause of failure."],["hs_database",""],["hs_expr_ext","A structure containing additional parameters related to an expression, passed in at build time to @ref hs_compile_ext_multi() or @ref hs_expression_ext_info."],["hs_expr_info","A type containing information related to an expression that is returned by @ref hs_expression_info() or @ref hs_expression_ext_info."],["hs_platform_info","A type containing information on the target platform which may optionally be provided to the compile calls (@ref hs_compile(), @ref hs_compile_multi(), @ref hs_compile_ext_multi())."],["hs_scratch",""],["hs_stream","Definition of the stream identifier type."]],"type":[["hs_alloc_t","The type of the callback function that will be used by Hyperscan to allocate more memory at runtime as required, for example in @ref hs_open_stream() to allocate stream state."],["hs_compile_error_t","A type containing error details that is returned by the compile calls (@ref hs_compile(), @ref hs_compile_multi() and @ref hs_compile_ext_multi()) on failure. The caller may inspect the values returned in this type to determine the cause of failure."],["hs_database_t","A Hyperscan pattern database."],["hs_error_t","A type for errors returned by Hyperscan functions."],["hs_expr_ext_t","A structure containing additional parameters related to an expression, passed in at build time to @ref hs_compile_ext_multi() or @ref hs_expression_ext_info."],["hs_expr_info_t","A type containing information related to an expression that is returned by @ref hs_expression_info() or @ref hs_expression_ext_info."],["hs_free_t","The type of the callback function that will be used by Hyperscan to free memory regions previously allocated using the @ref hs_alloc_t function."],["hs_platform_info_t","A type containing information on the target platform which may optionally be provided to the compile calls (@ref hs_compile(), @ref hs_compile_multi(), @ref hs_compile_ext_multi())."],["hs_scratch_t","A Hyperscan scratch space."],["hs_stream_t","The stream identifier returned by @ref hs_open_stream()."],["match_event_handler","Definition of the match event callback function type."]]});