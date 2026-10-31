use biver_core::environment::{Environment, LockOutcome};
use biver_std_environment::StdEnvironment;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime};
use std::{fs, io};

#[derive(Clone)]
pub struct TimeControlEnvironment {
    std_env: StdEnvironment,
    data: Rc<RefCell<Data>>,
}

struct Data {
    now: SystemTime,
    mtimes: HashMap<PathBuf, SystemTime>,
}

impl TimeControlEnvironment {
    pub fn new() -> TimeControlEnvironment {
        TimeControlEnvironment {
            std_env: StdEnvironment,
            data: Rc::new(RefCell::new(Data {
                now: SystemTime::now(),
                mtimes: HashMap::new(),
            })),
        }
    }

    pub fn advance_time(&mut self, duration: Duration) {
        self.data.borrow_mut().now += duration;
    }
}

impl Environment for TimeControlEnvironment {
    fn system_time_now(&self) -> SystemTime {
        self.data.borrow().now
    }

    fn file_mtime(&self, path: &Path) -> Result<SystemTime, io::Error> {
        let path = fs::canonicalize(path)?;
        let Some(mtime) = self.data.borrow().mtimes.get(&path).copied() else {
            panic!(
                "No mtime for {}. Mtimes: {:?}",
                path.display(),
                self.data.borrow().mtimes.keys()
            );
        };
        Ok(mtime)
    }

    fn file_exists(&self, path: &Path) -> Result<bool, io::Error> {
        self.std_env.file_exists(path)
    }

    fn read_file(&self, path: &Path) -> Result<File, io::Error> {
        self.std_env.read_file(path)
    }

    fn create_file(&self, path: &Path) -> Result<File, io::Error> {
        let result = self.std_env.create_file(path);
        let mut data = self.data.borrow_mut();
        let now = data.now;
        data.mtimes.insert(fs::canonicalize(path)?, now);
        result
    }

    fn overwrite_file(&self, path: &Path, contents: &str) -> Result<(), io::Error> {
        let result = self.std_env.overwrite_file(path, contents);
        let mut data = self.data.borrow_mut();
        let now = data.now;
        data.mtimes.insert(fs::canonicalize(path)?, now);
        result
    }

    fn overwrite_copy_file(&self, source: &Path, destination: &Path) -> Result<(), io::Error> {
        let result = self.std_env.overwrite_copy_file(source, destination);
        let mut data = self.data.borrow_mut();
        let now = data.now;
        data.mtimes.insert(fs::canonicalize(destination)?, now);
        result
    }

    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), io::Error> {
        let from_canon = fs::canonicalize(from)?;
        let result = self.std_env.rename_file(from, to);
        let mut data = self.data.borrow_mut();
        if let Some(mtime) = data.mtimes.remove(&from_canon) {
            data.mtimes.insert(fs::canonicalize(to)?, mtime);
        }
        result
    }

    fn remove_file(&self, path: &Path) -> Result<(), io::Error> {
        self.data
            .borrow_mut()
            .mtimes
            .remove(&fs::canonicalize(path)?);
        self.std_env.remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), io::Error> {
        self.std_env.create_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, io::Error> {
        self.std_env.read_dir(path)
    }

    fn try_lock_file(&self, path: &Path) -> Result<LockOutcome, io::Error> {
        self.std_env.try_lock_file(path)
    }

    fn unlock_file(&self, file: &File) -> Result<(), io::Error> {
        self.std_env.unlock_file(file)
    }

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, io::Error> {
        self.std_env.canonicalize_path(path)
    }
}
