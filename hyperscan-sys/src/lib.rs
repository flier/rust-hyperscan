//! Hyperscan is a software regular expression matching engine
//! designed with high performance and flexibility in mind.
#![no_std]
#![allow(non_camel_case_types, deref_nullptr)]

include!(concat!(env!("OUT_DIR"), "/hyperscan.rs"));

#[cfg(feature = "chimera")]
pub mod chimera {
    #![allow(deref_nullptr)]

    //! Chimera is a software regular expression matching engine that is a hybrid of Hyperscan and PCRE.
    //!
    //! The design goals of Chimera are to fully support PCRE syntax as well as to take advantage of
    //! the high performance nature of Hyperscan.
    include!(concat!(env!("OUT_DIR"), "/chimera.rs"));
}
