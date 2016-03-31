use std::ptr;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, BorrowMut};

use libc;

pub struct CPtr<T: Send>(*mut T);

impl<T: Send> fmt::Pointer for CPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:p}", self.0)
    }
}

impl<T: Send> fmt::Debug for CPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CPtr({:p})", self.0)
    }
}

impl<T: Send> CPtr<T> {
    #[inline]
    pub fn null() -> CPtr<T> {
        CPtr(ptr::null_mut())
    }

    #[inline]
    pub fn from_ptr(p: *mut T) -> CPtr<T> {
        CPtr(p)
    }
}

impl<T: Send> Borrow<T> for CPtr<T> {
    // the 'r lifetime results in the same semantics as `&*x` with Box<T>
    #[inline]
    fn borrow<'r>(&'r self) -> &'r T {
        // By construction, self.ptr is valid
        unsafe { &*self.0 }
    }
}

impl<T: Send> BorrowMut<T> for CPtr<T> {
    // the 'r lifetime results in the same semantics as `&*x` with Box<T>
    #[inline]
    fn borrow_mut<'r>(&'r mut self) -> &'r mut T {
        // By construction, self.ptr is valid
        unsafe { &mut *self.0 }
    }
}

impl<T: Send> Drop for CPtr<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            // Copy the object out from the pointer onto the stack,
            // where it is covered by normal Rust destructor semantics
            // and cleans itself up, if necessary
            ptr::read(self.0 as *const T);

            // clean-up our allocation
            libc::free(self.0 as *mut libc::c_void);

            self.0 = ptr::null_mut();
        }
    }
}

impl<T: Send> Deref for CPtr<T> {
    type Target = *mut T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Send> DerefMut for CPtr<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Send> AsRef<T> for CPtr<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

#[cfg(test)]
pub mod tests {
    use std::ptr;
    use std::mem;
    use std::borrow::Borrow;

    use libc;
    use regex::Regex;

    use super::*;

    struct Foo {
        bar: u32,
    }

    fn validate_borrow<T: Borrow<Foo>>(b: T) {
        assert_eq!(b.borrow().bar, 32);
    }


    #[test]
    fn test_borrow() {
        let p = CPtr::<Foo>::new(Foo { bar: 32 });

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
            assert_eq!((*p.0).bar, 32);

            assert!(Regex::new(r"CPtr\(\w+\)")
                        .unwrap()
                        .is_match(&format!("{:?}", p)));
        }
    }
}
