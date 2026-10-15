use std::fs::File;
use std::io;
use std::io::{BufReader, Read};
use std::path::Path;
use xxhash_rust::xxh3::Xxh3;

pub fn read(file: impl Read) -> io::Result<u128> {
    let mut reader = BufReader::new(file);
    let mut hasher = Xxh3::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.digest128())
}

pub fn from_file_path(file: &Path) -> io::Result<u128> {
    read(File::open(file)?)
}
