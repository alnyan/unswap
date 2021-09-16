use crate::{Error, OsImpl};
use std::ffi::c_void;
use std::ptr::null_mut;

pub(crate) struct UnixImpl;

unsafe impl OsImpl for UnixImpl {
    fn alloc_pages(size: usize) -> Result<*mut c_void, Error> {
        if size & 0xFFF != 0 {
            return Err(Error::AlignError);
        }
        let pages = unsafe {
            libc::mmap(
                null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if pages == libc::MAP_FAILED {
            return Err(Error::OsError);
        }
        if unsafe { libc::mlock(pages, size) } != 0 {
            return Err(Error::OsError);
        }

        Ok(pages)
    }

    unsafe fn free_pages(at: *mut c_void, size: usize) {
        libc::munmap(at, size);
    }
}
