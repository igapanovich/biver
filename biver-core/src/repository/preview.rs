use crate::error::Result;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::{fs, io};

pub struct Preview {
    file: Option<File>,
    path: PathBuf,
    cleanup_required: bool,
}

impl Preview {
    pub fn new(path: PathBuf) -> Result<Preview> {
        Ok(Preview {
            file: Some(File::open(&path)?),
            path,
            cleanup_required: false,
        })
    }

    pub fn with_cleanup(mut self) -> Preview {
        self.cleanup_required = true;
        self
    }
}

impl Read for Preview {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(file) = &mut self.file else {
            return Err(io::Error::from(io::ErrorKind::BrokenPipe));
        };

        file.read(buf)
    }
}

impl Seek for Preview {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let Some(file) = &mut self.file else {
            return Err(io::Error::from(io::ErrorKind::BrokenPipe));
        };

        file.seek(pos)
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        if self.cleanup_required {
            self.file = None;
            let _ = fs::remove_file(&self.path);
        }
    }
}
