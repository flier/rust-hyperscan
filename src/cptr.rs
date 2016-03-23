use std::mem;
use std::ptr;
use std::ops::Deref;
use std::borrow::{Borrow, BorrowMut};

use libc;

pub struct CPtr<T: Send> {
    p: *mut T,
}

impl<T: Send> CPtr<T> {
    pub fn new(value: T) -> CPtr<T> {
        unsafe {
            let ptr = libc::malloc(mem::size_of::<T>() as libc::size_t) as *mut T;

            // we *need* valid pointer.
            assert!(!ptr.is_null());

            // `*ptr` is uninitialized, and `*ptr = value` would
            // attempt to destroy it `overwrite` moves a value into
            // this memory without attempting to drop the original
            // value.
            ptr::write(&mut *ptr, value);

            CPtr { p: ptr }
        }
    }

    pub fn from_ptr(p: *mut T) -> CPtr<T> {
        CPtr { p: p }
    }
}

impl<T: Send> Borrow<T> for CPtr<T> {
    // the 'r lifetime results in the same semantics as `&*x` with
    // Box<T>
    fn borrow<'r>(&'r self) -> &'r T {
        // By construction, self.ptr is valid
        unsafe { &*self.p }
    }
}

impl<T: Send> BorrowMut<T> for CPtr<T> {
    // the 'r lifetime results in the same semantics as `&*x` with
    // Box<T>
    fn borrow_mut<'r>(&'r mut self) -> &'r mut T {
        // By construction, self.ptr is valid
        unsafe { &mut *self.p }
    }
}

impl<T: Send> Drop for CPtr<T> {
    fn drop(&mut self) {
        unsafe {
            // Copy the object out from the pointer onto the stack,
            // where it is covered by normal Rust destructor semantics
            // and cleans itself up, if necessary
            ptr::read(self.p as *const T);

            // clean-up our allocation
            libc::free(self.p as *mut libc::c_void);

            self.p = ptr::null_mut();
        }
    }
}

impl<T: Send> Deref for CPtr<T> {
    type Target = *mut T;

    fn deref(&self) -> &*mut T {
        &self.p
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;
    use std::mem;
    use std::borrow::{Borrow, BorrowMut};
    use libc;

    use super::*;

    struct Foo {
        bar: u32,
    }

    fn validate_borrow<T: Borrow<Foo>>(b: T) {
        assert_eq!(b.borrow().bar, 32);
    }


    #[test]
    fn test_borrow() {
        let mut p = CPtr::<Foo>::new(Foo { bar: 32 });

        assert!(*p != ptr::null_mut());

        validate_borrow(p);
    }

    #[test]
    fn test_from_ptr() {
        unsafe {
            let foo = libc::malloc(mem::size_of::<Foo>() as libc::size_t) as *mut Foo;

            (*foo).bar = 32;

            let p = CPtr::<Foo>::from_ptr(foo);

            assert!(*p != ptr::null_mut());
            assert_eq!((**p).bar, 32);
        }
    }
}
