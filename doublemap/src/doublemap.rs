use nix::sys::memfd::{MFdFlags, memfd_create};
use nix::sys::mman::{self, MapFlags, ProtFlags, munmap};
use nix::unistd::ftruncate;

use std::ffi::CStr;
use std::fmt;
use std::num::NonZeroUsize;
use std::os::fd::OwnedFd;
use std::ptr::NonNull;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DoublemapError {
    #[error("Size of T cannot be zero")]
    ZeroSizedType,
    #[error("Overflow")]
    Overflow,
    #[error("{0}")]
    Nix(#[from] nix::Error),
}

pub struct Doublemap<T> {
    ptr: NonNull<T>,
    // len of one segment in elements, we map 2x to get the mirror
    capacity: usize,
    // OwnedFd closes on drop — keeps the memfd alive as long as the mappings exist
    _fd: OwnedFd,
}

impl<T> fmt::Debug for Doublemap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Doublemap")
            .field("ptr", &self.ptr)
            .field("capacity", &self.capacity)
            .finish()
    }
}

/// Capacity is in number of elements, not bytes. The actual capacity
/// is rounded up to the nearest page size.
impl<T: Copy> Doublemap<T> {
    pub fn new(capacity: usize) -> Result<Self, DoublemapError> {
        let size_of_t = std::mem::size_of::<T>();
        if size_of_t == 0 {
            return Err(DoublemapError::ZeroSizedType);
        }

        let capacity_bytes = capacity
            .checked_mul(size_of_t)
            .ok_or(DoublemapError::Overflow)?;

        let page_size = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) } as usize;

        // Round up to page boundary
        let aligned = capacity_bytes.div_ceil(page_size) * page_size;
        let total = NonZeroUsize::new(aligned * 2).ok_or(DoublemapError::Overflow)?;
        let half = NonZeroUsize::new(aligned).ok_or(DoublemapError::Overflow)?;

        // Create anonymous file and size it
        let name = CStr::from_bytes_with_nul(b"doublemap\0").unwrap();
        let fd = memfd_create(name, MFdFlags::empty())?;
        ftruncate(&fd, aligned as i64)?;

        // Reserve contiguous virtual address space (PROT_NONE = no access yet)
        let reserved = unsafe {
            mman::mmap_anonymous(None, total, ProtFlags::PROT_NONE, MapFlags::MAP_PRIVATE)?
        };

        let rw = ProtFlags::PROT_READ | ProtFlags::PROT_WRITE;
        let shared_fixed = MapFlags::MAP_SHARED | MapFlags::MAP_FIXED;

        // Convert NonNull<c_void> to NonZeroUsize for mmap's addr parameter
        let reserved_addr = NonZeroUsize::new(reserved.as_ptr() as usize).unwrap();

        // First half: map the memfd at the start of the reserved region
        let first = unsafe { mman::mmap(Some(reserved_addr), half, rw, shared_fixed, &fd, 0)? };

        // Second half (mirror): map the same memfd right after the first half
        let mirror_addr = NonZeroUsize::new(first.as_ptr() as usize + aligned).unwrap();
        let second = unsafe { mman::mmap(Some(mirror_addr), half, rw, shared_fixed, &fd, 0) };

        if let Err(e) = second {
            // Clean up the entire reserved region on failure
            unsafe { munmap(reserved, aligned * 2) }?;
            return Err(DoublemapError::Nix(e));
        }

        Ok(Self {
            ptr: first.cast(),
            capacity: aligned / size_of_t,
            _fd: fd,
        })
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.capacity * 2) }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.capacity * 2) }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// SAFETY: Doublemap owns the mmap'd region exclusively (no aliasing).
// It is safe to send/share if T is, just like Vec<T>.
unsafe impl<T: Send> Send for Doublemap<T> {}
unsafe impl<T: Sync> Sync for Doublemap<T> {}

impl<T> Drop for Doublemap<T> {
    fn drop(&mut self) {
        unsafe {
            let size_bytes = self.capacity * std::mem::size_of::<T>() * 2;
            let _ = munmap(self.ptr.cast(), size_bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use num_complex::Complex32;

    #[test]
    fn test_doublemap() {
        let capacity = 1024;
        let doublemap = Doublemap::<u8>::new(capacity).unwrap();
        assert_eq!(doublemap.capacity(), 4096);
    }

    #[test]
    fn test_slice() {
        let capacity = 4096;
        let mut doublemap = Doublemap::<u8>::new(capacity).unwrap();

        let slice = doublemap.as_mut_slice();
        assert_eq!(slice.len(), 8192);

        let len = slice.len() / 2;

        // Loop and fill with i % 256
        (0..len).for_each(|i| {
            slice[i] = (i % 256) as u8;
        });

        // Check that the first and second halves are the same
        for i in 0..len {
            assert_eq!(slice[i], slice[i + len]);
        }
    }

    #[test]
    fn test_complex_i16() {
        let mut map = Doublemap::<i16>::new(1024).unwrap();
        let slice = map.as_mut_slice();

        assert_eq!(slice.len(), 4096);

        for i in 0..slice.len() / 2 {
            slice[i] = (i as i16) * 2;
        }

        for i in 0..slice.len() / 2 {
            assert_eq!(slice[i], slice[i + slice.len() / 2]);
        }
    }

    #[test]
    fn test_complex_buffer() {
        let mut map = Doublemap::<Complex32>::new(2048).unwrap();
        let slice = map.as_mut_slice();

        assert_eq!(slice.len(), 4096);

        for i in 0..slice.len() / 2 {
            slice[i] = Complex32::new(i as f32, (i + 1) as f32);
        }

        for i in 0..slice.len() / 2 {
            assert_eq!(slice[i], slice[i + slice.len() / 2]);
        }
    }
}
