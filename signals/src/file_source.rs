use num_complex::Complex32;
use std::path::PathBuf;

use std::io::Read;
use std::io::{self, Seek};

use std::io::BufReader;

pub struct FileSource {
    reader: BufReader<std::fs::File>,
}

impl FileSource {
    pub fn new(path: PathBuf) -> Result<Self, std::io::Error> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        Ok(Self { reader })
    }

    pub fn reset(&mut self) -> Result<(), std::io::Error> {
        self.reader.rewind()?;
        Ok(())
    }
}

impl Iterator for FileSource {
    type Item = Complex32;

    fn next(&mut self) -> Option<Self::Item> {
        // Read std::mem::size_of::<Complex32>() bytes from the file
        let mut buffer = [0u8; std::mem::size_of::<Complex32>()];

        match self.reader.read_exact(&mut buffer) {
            Ok(_) => {
                let complex: Complex32 = unsafe {
                    // Cast the byte slice to a pointer to Complex32
                    // This assumes the memory layout of the buffer matches that of Complex32
                    std::ptr::read(buffer.as_ptr() as *const Complex32)
                };

                Some(complex)
            }
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // EOF reached, reset and continue reading from the start
                self.reset().ok();
                self.next() // Recursively try to read again from the start
            }
            Err(_) => None, // Any other error
        }
    }
}
