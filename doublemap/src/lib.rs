pub mod ringbuffer;

use libc::{
    close, ftruncate, memfd_create, mmap, munmap, sysconf, MAP_ANON, MAP_FAILED, MAP_FIXED,
    MAP_PRIVATE, MAP_SHARED, PROT_NONE, PROT_READ, PROT_WRITE,
};
use std::ffi::CString;
use std::io;
use std::ptr;
use std::ptr::NonNull;

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Doublemap<T> {
    ptr: NonNull<T>,
    // len of one segment, we map 2x len to get mirror
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

/// Capacity is in number of elements, not bytes We will need to
/// allocated 2x the requested capacity x size_of(T) rounded up to the
/// nearest page size.
impl<T: Copy> Doublemap<T> {
    pub fn pagesize() -> usize {
        // Get system page size
        unsafe { sysconf(libc::_SC_PAGESIZE) as usize }
    }

    pub fn new(capacity: usize) -> io::Result<Self> {
        // Size of T
        let size_of_t = std::mem::size_of::<T>();
        if size_of_t == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Size of T cannot be zero",
            ));
        }

        // Calculate the total capacity needed in bytes
        let capacity = capacity.checked_mul(size_of_t).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Capacity overflow when calculating total size",
            )
        })?;

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
            ptr: NonNull::new(buffer_1 as *mut T)
                .expect("mmap returned null pointer, which should not happen"),
            len: aligned_capacity / size_of_t,
            mem_fd,
            //_phantom: PhantomData,
        })
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len * 2) }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len * 2) }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<T> Drop for Doublemap<T> {
    fn drop(&mut self) {
        unsafe {
            let size_of_t = std::mem::size_of::<T>();
            munmap(
                self.ptr.as_ptr() as *mut libc::c_void,
                self.len * size_of_t * 2,
            );
            close(self.mem_fd);
        }
    }
}

// Test
#[cfg(test)]
mod tests {
    use super::*;

    use num_complex::Complex32;

    #[test]
    fn test_doublemap() {
        let capacity = 1024;
        let doublemap = Doublemap::<u8>::new(capacity).unwrap();
        assert_eq!(doublemap.len(), 4096);
    }

    #[test]
    fn test_slice() {
        let capacity = 4096;
        let mut doublemap = Doublemap::<u8>::new(capacity).unwrap();

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

    // Test i16
    #[test]
    fn test_complex_i16() {
        let mut map = Doublemap::<i16>::new(1024).unwrap();
        let slice = map.as_mut_slice();

        // Check len
        assert_eq!(slice.len(), 4096);

        // Fill the first half with complex numbers
        for i in 0..slice.len() / 2 {
            slice[i] = (i as i16) * 2;
        }

        // Check that the second half is the same as the first half
        for i in 0..slice.len() / 2 {
            assert_eq!(slice[i], slice[i + slice.len() / 2]);
        }
    }

    #[test]
    fn test_complex_buffer() {
        let mut map = Doublemap::<Complex32>::new(2048).unwrap();
        let slice = map.as_mut_slice();

        // Check len
        assert_eq!(slice.len(), 4096);

        // Fill the first half with complex numbers
        for i in 0..slice.len() / 2 {
            slice[i] = Complex32::new(i as f32, (i + 1) as f32);
        }

        // Check that the second half is the same as the first half
        for i in 0..slice.len() / 2 {
            assert_eq!(slice[i], slice[i + slice.len() / 2]);
        }
    }
}
