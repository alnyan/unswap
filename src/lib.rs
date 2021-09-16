//! This crate provides a way of allocating data buffers
//! protected from being swapped out in low-memory conditions,
//! which is required for storing secret data.
//!
//! # Current problems
//!
//! Currently, the crate aligns all allocation sizes to
//! a page boundary, which may be highly inefficient for
//! making lots of small allocations. This, however, should not
//! be a problem for typical use-cases: storing and
//! manipulating private key data and other secrets.
#![deny(missing_docs)]

#[macro_use]
extern crate cfg_if;

use std::alloc::Layout;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::slice;

/// OS-specific memory management trait
pub unsafe trait OsImpl {
    /// Allocates a region of `size` bytes, aligned to a
    /// page boundary and protected from being swapped out
    /// onto a disk.
    ///
    /// Will panic if `size` is not page-aligned.
    fn alloc_pages(size: usize) -> Result<*mut c_void, Error>;

    /// Releases allocated pages back.
    ///
    /// # Safety
    ///
    /// The function is unsafe because it does not perform
    /// validation of `at` and `size`, which needs to be
    /// done manually.
    ///
    /// `at` must be a valid and properly (page-) aligned
    /// address, `size` must be correct and aligned as well.
    unsafe fn free_pages(at: *mut c_void, size: usize);
}

/// Errors related to allocating and locking memory buffers
#[derive(Debug)]
pub enum Error {
    /// Size given was not properly aligned
    AlignError,
    /// The memory allocation routine failed
    OsError,
}

cfg_if! {
    if #[cfg(target_os = "linux")] {
        extern crate libc;

        mod impl_unix;
        use impl_unix::UnixImpl as Impl;
    }
}

/// Array residing in non-swappable memory
pub struct UnswapArray<T> {
    data: *mut c_void,
    len: usize,
    size: usize,
    _pd: PhantomData<T>,
}

/// Container for unswappable data

impl<T: Clone> UnswapArray<T> {
    /// Allocates a new array for `len` elements of type `T`.
    ///
    /// The resulting array is page-aligned.
    pub fn new(value: T, len: usize) -> Self {
        let layout = Layout::array::<T>(len).unwrap();
        if layout.align() > 0x1000 {
            unimplemented!();
        }
        let size = (layout.size() + 0xFFF) & !0xFFF;
        let data = Impl::alloc_pages(size).expect("Failed to allocate locked memory pages");

        let array: &mut [MaybeUninit<T>] =
            unsafe { slice::from_raw_parts_mut(data as *mut MaybeUninit<T>, len) };
        for uninit in array.iter_mut() {
            uninit.write(value.clone());
        }

        Self {
            data,
            len,
            size,
            _pd: PhantomData,
        }
    }
}

impl<T> Drop for UnswapArray<T> {
    fn drop(&mut self) {
        unsafe {
            Impl::free_pages(self.data, self.size);
        }
    }
}

impl<T> Deref for UnswapArray<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.data as *const T, self.len) }
    }
}

impl<T> DerefMut for UnswapArray<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.data as *mut T, self.len) }
    }
}
