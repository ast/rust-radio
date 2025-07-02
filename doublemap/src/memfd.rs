use std::ffi::CString;

use libc::{
    MAP_ANON, MAP_FAILED, MAP_FIXED, MAP_PRIVATE, MAP_SHARED, PROT_NONE, PROT_READ, PROT_WRITE,
    close, ftruncate, memfd_create, mmap, munmap, sysconf,
};

use std::os::unix::io::{AsFd, BorrowedFd};

pub struct Memfd {
    fd: i32,
}

impl Memfd {
    pub fn new(name: &str, size: usize) -> Result<Self, std::io::Error> {
        let c_name = CString::new(name)?;
        let fd = unsafe { memfd_create(c_name.as_ptr(), 0) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        if unsafe { ftruncate(fd, size as i64) } != 0 {
            unsafe { close(fd) };
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self { fd })
    }

    pub fn fd(&self) -> i32 {
        self.fd
    }
}

impl Drop for Memfd {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

impl AsFd for Memfd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        // SAFETY: self.fd is a valid open file descriptor owned by this struct
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}
