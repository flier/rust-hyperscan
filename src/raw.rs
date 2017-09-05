#![allow(non_camel_case_types, dead_code)]

include!(concat!(env!("OUT_DIR"), "/raw_bindgen.rs"));

pub trait AsPtr {
    type Type;

    fn as_ptr(&self) -> *const Self::Type;
}

impl<T> AsPtr for *const T {
    type Type = T;

    fn as_ptr(&self) -> *const Self::Type {
        *self
    }
}

impl<T> AsPtr for *mut T {
    type Type = T;

    fn as_ptr(&self) -> *const Self::Type {
        *self
    }
}

pub trait AsMutPtr: AsPtr {
    fn as_mut_ptr(&mut self) -> *mut Self::Type;
}

impl<T> AsMutPtr for *mut T {
    fn as_mut_ptr(&mut self) -> *mut Self::Type {
        *self
    }
}

pub trait IntoRaw {
    type Type;

    fn into_raw(self) -> *mut Self::Type;
}
