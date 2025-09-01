use std::ffi::CString;

use libc::{close, ftruncate, memfd_create};

#[derive(Debug)]
pub struct Memfd {
    fd: i32,
}

// Memfd is a wrapper around a memory file descriptor
// to make sure it's dropped.
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
