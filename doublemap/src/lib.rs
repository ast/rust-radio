use libc::{
    c_void, close, ftruncate, memfd_create, mmap, munmap, sysconf, MAP_ANON, MAP_FAILED, MAP_FIXED,
    MAP_PRIVATE, MAP_SHARED, PROT_NONE, PROT_READ, PROT_WRITE,
};
use std::ffi::CString;
use std::io;
use std::ptr;

use std::time::{SystemTime, UNIX_EPOCH};

pub struct Doublemap {
    ptr: *mut libc::c_void,
    len: usize,
    mem_fd: i32,
}
fn make_memfd_name(size: usize) -> CString {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    // Construct something like: "doublemap.4096.1715877445000"
    let name = format!("doublemap.{}.{}", size, timestamp);

    // Convert to CString for FFI use
    CString::new(name).expect("CString::new failed (null byte in string?)")
}

impl Doublemap {
    pub fn new(capacity: usize) -> io::Result<Self> {
        // Get system page size
        let page_size = unsafe { sysconf(libc::_SC_PAGESIZE) } as usize;

        // Round up requested capacity to the nearest page size
        let aligned_capacity = capacity.div_ceil(page_size) * page_size;

        let name = make_memfd_name(aligned_capacity);

        // Create a memfd
        let mem_fd = unsafe { memfd_create(name.as_ptr(), 0) };
        if mem_fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // Resize it
        if unsafe { ftruncate(mem_fd, aligned_capacity as i64) } != 0 {
            unsafe { close(mem_fd) };
            return Err(io::Error::last_os_error());
        }

        // Reserve 2x space with PROT_NONE
        let total_size = aligned_capacity * 2;
        let reserved = unsafe {
            mmap(
                ptr::null_mut(),
                total_size,
                PROT_NONE,
                MAP_PRIVATE | MAP_ANON,
                -1,
                0,
            )
        };
        if reserved == MAP_FAILED {
            unsafe { close(mem_fd) };
            return Err(io::Error::last_os_error());
        }

        // First mapping
        let addr_hint_1 = reserved;
        let buffer_1 = unsafe {
            mmap(
                addr_hint_1,
                aligned_capacity,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_FIXED,
                mem_fd,
                0,
            )
        };
        if buffer_1 != addr_hint_1 || buffer_1 == MAP_FAILED {
            unsafe {
                munmap(reserved, total_size);
                close(mem_fd);
            }
            return Err(io::Error::last_os_error());
        }

        // Second mapping (mirror)
        let addr_hint_2 = (buffer_1 as usize + aligned_capacity) as *mut libc::c_void;
        let buffer_2 = unsafe {
            mmap(
                addr_hint_2,
                aligned_capacity,
                PROT_READ | PROT_WRITE,
                MAP_SHARED | MAP_FIXED,
                mem_fd,
                0,
            )
        };
        if buffer_2 != addr_hint_2 || buffer_2 == MAP_FAILED {
            unsafe {
                munmap(buffer_1, aligned_capacity);
                munmap(reserved, total_size);
                close(mem_fd);
            }
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            ptr: buffer_1,
            len: aligned_capacity,
            mem_fd,
        })
    }

    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr as *mut u8
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr(), self.len * 2) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.as_ptr(), self.len * 2) }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl Drop for Doublemap {
    fn drop(&mut self) {
        unsafe {
            munmap(self.ptr, self.len * 2);
            close(self.mem_fd);
        }
    }
}

// Test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doublemap() {
        let capacity = 1024;
        let doublemap = Doublemap::new(capacity).unwrap();
        assert_eq!(doublemap.len(), 4096);
        assert!(!doublemap.as_ptr().is_null());
    }

    #[test]
    fn test_slice() {
        let capacity = 4096;
        let mut doublemap = Doublemap::new(capacity).unwrap();

        assert!(!doublemap.as_ptr().is_null());

        let slice = doublemap.as_mut_slice();
        assert_eq!(slice.len(), 8192);

        let len = slice.len() / 2;

        // Loop and fill with i % 256
        for i in 0..len {
            slice[i] = (i % 256) as u8;
        }

        // Check that the first and second halves are the same
        for i in 0..len {
            assert_eq!(slice[i], slice[i + len]);
        }
    }
}
