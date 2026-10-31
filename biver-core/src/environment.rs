use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub trait Environment {
    fn system_time_now(&self) -> SystemTime;

    fn file_mtime(&self, path: &Path) -> Result<SystemTime, io::Error>;

    fn file_exists(&self, path: &Path) -> Result<bool, io::Error>;

    fn read_file(&self, path: &Path) -> Result<File, io::Error>;

    fn read_file_to_string(&self, path: &Path) -> Result<String, io::Error> {
        let mut str = String::new();
        self.read_file(path)?.read_to_string(&mut str)?;
        Ok(str)
    }

    fn read_file_to_end(&self, path: &Path) -> Result<Vec<u8>, io::Error> {
        let mut bytes = Vec::new();
        self.read_file(path)?.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    fn create_file(&self, path: &Path) -> Result<File, io::Error>;

    fn overwrite_file(&self, path: &Path, contents: &str) -> Result<(), io::Error>;

    fn overwrite_copy_file(&self, source: &Path, destination: &Path) -> Result<(), io::Error>;

    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), io::Error>;

    fn remove_file(&self, path: &Path) -> Result<(), io::Error>;

    fn create_dir_all(&self, path: &Path) -> Result<(), io::Error>;

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, io::Error>;

    fn try_lock_file(&self, path: &Path) -> Result<LockOutcome, io::Error>;

    fn unlock_file(&self, file: &File) -> Result<(), io::Error>;

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, io::Error>;
}

pub enum LockOutcome {
    Acquired(File),
    Locked,
}
