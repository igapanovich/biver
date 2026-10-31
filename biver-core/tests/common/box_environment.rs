use biver_core::environment::{Environment, LockOutcome};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct BoxEnvironment(Box<dyn CloneEnvironment>);

impl BoxEnvironment {
    pub fn new(inner: impl Environment + Clone + 'static) -> BoxEnvironment {
        BoxEnvironment(Box::new(inner))
    }
}

impl Environment for BoxEnvironment {
    fn system_time_now(&self) -> SystemTime {
        self.0.system_time_now()
    }

    fn file_mtime(&self, path: &Path) -> Result<SystemTime, io::Error> {
        self.0.file_mtime(path)
    }

    fn file_exists(&self, path: &Path) -> Result<bool, io::Error> {
        self.0.file_exists(path)
    }

    fn read_file(&self, path: &Path) -> Result<File, io::Error> {
        self.0.read_file(path)
    }

    fn read_file_to_string(&self, path: &Path) -> Result<String, io::Error> {
        self.0.read_file_to_string(path)
    }

    fn read_file_to_end(&self, path: &Path) -> Result<Vec<u8>, io::Error> {
        self.0.read_file_to_end(path)
    }

    fn create_file(&self, path: &Path) -> Result<File, io::Error> {
        self.0.create_file(path)
    }

    fn overwrite_file(&self, path: &Path, contents: &str) -> Result<(), io::Error> {
        self.0.overwrite_file(path, contents)
    }

    fn overwrite_copy_file(&self, source: &Path, destination: &Path) -> Result<(), io::Error> {
        self.0.overwrite_copy_file(source, destination)
    }

    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), io::Error> {
        self.0.rename_file(from, to)
    }

    fn remove_file(&self, path: &Path) -> Result<(), io::Error> {
        self.0.remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), io::Error> {
        self.0.create_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, io::Error> {
        self.0.read_dir(path)
    }

    fn try_lock_file(&self, path: &Path) -> Result<LockOutcome, io::Error> {
        self.0.try_lock_file(path)
    }

    fn unlock_file(&self, file: &File) -> Result<(), io::Error> {
        self.0.unlock_file(file)
    }

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, io::Error> {
        self.0.canonicalize_path(path)
    }
}

impl Clone for BoxEnvironment {
    fn clone(&self) -> BoxEnvironment {
        BoxEnvironment(self.0.clone_box())
    }
}

trait CloneEnvironment: Environment {
    fn clone_box(&self) -> Box<dyn CloneEnvironment>;
}

impl<T: Environment + Clone + 'static> CloneEnvironment for T {
    fn clone_box(&self) -> Box<dyn CloneEnvironment> {
        Box::new(self.clone())
    }
}
