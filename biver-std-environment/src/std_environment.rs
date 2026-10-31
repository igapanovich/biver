use biver_core::environment::{Environment, LockOutcome};
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fs, io};

#[derive(Debug, Copy, Clone)]
pub struct StdEnvironment;

impl Environment for StdEnvironment {
    fn system_time_now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn file_mtime(&self, path: &Path) -> Result<SystemTime, io::Error> {
        fs::metadata(path)?.modified()
    }

    fn file_exists(&self, path: &Path) -> Result<bool, io::Error> {
        fs::exists(path)
    }

    fn read_file(&self, path: &Path) -> Result<File, io::Error> {
        File::open(path)
    }

    fn read_file_to_string(&self, path: &Path) -> Result<String, io::Error> {
        fs::read_to_string(path)
    }

    fn read_file_to_end(&self, path: &Path) -> Result<Vec<u8>, io::Error> {
        fs::read(path)
    }

    fn create_file(&self, path: &Path) -> Result<File, io::Error> {
        File::create(path)
    }

    fn overwrite_file(&self, path: &Path, contents: &str) -> Result<(), io::Error> {
        fs::write(path, contents)?;
        Ok(())
    }

    fn overwrite_copy_file(&self, source: &Path, destination: &Path) -> Result<(), io::Error> {
        fs::copy(source, destination)?;
        Ok(())
    }

    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), io::Error> {
        fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> Result<(), io::Error> {
        fs::remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), io::Error> {
        fs::create_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, io::Error> {
        let paths = fs::read_dir(path)?
            .collect::<Result<Vec<_>, io::Error>>()?
            .into_iter()
            .map(|e| e.path())
            .collect();
        Ok(paths)
    }

    fn try_lock_file(&self, path: &Path) -> Result<LockOutcome, io::Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        match file.try_lock() {
            Ok(()) => Ok(LockOutcome::Acquired(file)),
            Err(TryLockError::WouldBlock) => Ok(LockOutcome::Locked),
            Err(e) => Err(io::Error::from(e)),
        }
    }

    fn unlock_file(&self, file: &File) -> Result<(), io::Error> {
        file.unlock()
    }

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, io::Error> {
        fs::canonicalize(path)
    }
}
