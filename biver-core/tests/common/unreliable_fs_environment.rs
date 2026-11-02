use crate::common::box_environment::BoxEnvironment;
use biver_core::environment::{Environment, LockOutcome};
use rand::prelude::*;
use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

#[derive(Clone)]
pub struct UnreliableFsEnvironment<Inner> {
    inner: Inner,
    rng: Rc<RefCell<StdRng>>,
}

impl<Inner: Environment> UnreliableFsEnvironment<Inner> {
    pub fn new(inner: Inner) -> Self {
        UnreliableFsEnvironment {
            inner,
            rng: Rc::new(RefCell::new(StdRng::seed_from_u64(1))),
        }
    }

    pub fn inner(&self) -> &Inner {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut Inner {
        &mut self.inner
    }

    fn maybe_fail(&self) -> Result<(), io::Error> {
        if !self.rng.borrow_mut().next_u32().is_multiple_of(10) {
            return Ok(());
        }

        let kind = match self.rng.borrow_mut().next_u32() % 39 {
            0 => ErrorKind::NotFound,
            1 => ErrorKind::PermissionDenied,
            2 => ErrorKind::ConnectionRefused,
            3 => ErrorKind::ConnectionReset,
            4 => ErrorKind::HostUnreachable,
            5 => ErrorKind::NetworkUnreachable,
            6 => ErrorKind::ConnectionAborted,
            7 => ErrorKind::NotConnected,
            8 => ErrorKind::AddrInUse,
            9 => ErrorKind::AddrNotAvailable,
            10 => ErrorKind::NetworkDown,
            11 => ErrorKind::BrokenPipe,
            12 => ErrorKind::AlreadyExists,
            13 => ErrorKind::WouldBlock,
            14 => ErrorKind::NotADirectory,
            15 => ErrorKind::IsADirectory,
            16 => ErrorKind::DirectoryNotEmpty,
            17 => ErrorKind::ReadOnlyFilesystem,
            18 => ErrorKind::StaleNetworkFileHandle,
            19 => ErrorKind::InvalidInput,
            20 => ErrorKind::InvalidData,
            21 => ErrorKind::TimedOut,
            22 => ErrorKind::WriteZero,
            23 => ErrorKind::StorageFull,
            24 => ErrorKind::NotSeekable,
            25 => ErrorKind::QuotaExceeded,
            26 => ErrorKind::FileTooLarge,
            27 => ErrorKind::ResourceBusy,
            28 => ErrorKind::ExecutableFileBusy,
            29 => ErrorKind::Deadlock,
            30 => ErrorKind::CrossesDevices,
            31 => ErrorKind::TooManyLinks,
            32 => ErrorKind::InvalidFilename,
            33 => ErrorKind::ArgumentListTooLong,
            34 => ErrorKind::Interrupted,
            35 => ErrorKind::Unsupported,
            36 => ErrorKind::UnexpectedEof,
            37 => ErrorKind::OutOfMemory,
            _ => ErrorKind::Other,
        };

        Err(io::Error::new(kind, "UnreliableFS fail"))
    }
}

impl<Inner: Environment> Environment for UnreliableFsEnvironment<Inner> {
    fn system_time_now(&self) -> SystemTime {
        self.inner.system_time_now()
    }

    fn file_mtime(&self, path: &Path) -> Result<SystemTime, io::Error> {
        self.maybe_fail()?;
        self.inner.file_mtime(path)
    }

    fn file_exists(&self, path: &Path) -> Result<bool, io::Error> {
        self.maybe_fail()?;
        self.inner.file_exists(path)
    }

    fn read_file(&self, path: &Path) -> Result<File, io::Error> {
        self.maybe_fail()?;
        self.inner.read_file(path)
    }

    fn read_file_to_string(&self, path: &Path) -> Result<String, io::Error> {
        self.maybe_fail()?;
        self.inner.read_file_to_string(path)
    }

    fn read_file_to_end(&self, path: &Path) -> Result<Vec<u8>, io::Error> {
        self.maybe_fail()?;
        self.inner.read_file_to_end(path)
    }

    fn create_file(&self, path: &Path) -> Result<File, io::Error> {
        self.maybe_fail()?;
        self.inner.create_file(path)
    }

    fn overwrite_file(&self, path: &Path, contents: &str) -> Result<(), io::Error> {
        self.maybe_fail()?;
        self.inner.overwrite_file(path, contents)
    }

    fn overwrite_copy_file(&self, source: &Path, destination: &Path) -> Result<(), io::Error> {
        self.maybe_fail()?;
        self.inner.overwrite_copy_file(source, destination)
    }

    fn rename_file(&self, from: &Path, to: &Path) -> Result<(), io::Error> {
        self.maybe_fail()?;
        self.inner.rename_file(from, to)
    }

    fn remove_file(&self, path: &Path) -> Result<(), io::Error> {
        self.maybe_fail()?;
        self.inner.remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), io::Error> {
        self.maybe_fail()?;
        self.inner.create_dir_all(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, io::Error> {
        self.maybe_fail()?;
        self.inner.read_dir(path)
    }

    fn try_lock_file(&self, path: &Path) -> Result<LockOutcome, io::Error> {
        self.maybe_fail()?;
        self.inner.try_lock_file(path)
    }

    fn unlock_file(&self, file: &File) -> Result<(), io::Error> {
        self.maybe_fail()?;
        self.inner.unlock_file(file)
    }

    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf, io::Error> {
        self.maybe_fail()?;
        self.inner.canonicalize_path(path)
    }
}
