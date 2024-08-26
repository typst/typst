//! TODO: write some docs?

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ecow::eco_format;
use fs4::fs_std::FileExt;
use typst::diag::PackageError;
use typst::syntax::package::PackageSpec;

pub struct LockFile {
    path: PathBuf,
    tmp_path: String,
    /// Was the lock acquired on the first try.
    acquired_first_try: Option<bool>,
    file: fs::File,
}

// Where do I put it and who is gonna use it?
/// Returns mapped `io::Error` to `PackageError::Other` with a custom message.
/// Intended to be used inside `io::Result`'s `.map_err()` method.
fn other_err(message: &'static str) -> impl Fn(io::Error) -> PackageError {
    move |error| PackageError::Other(Some(eco_format!("{message}: {error}")))
}

// Only 2 methods are public. If this must change look into code inside did_wait.
impl LockFile {
    /// `packages_root_dir` is normally either the cache or the local directory.
    fn new(packages_root_dir: &Path, spec: &PackageSpec) -> Result<Self, PackageError> {
        // Directory that is 1 level above the package directory.
        let parent_dir_path = format!("{}/{}", spec.namespace, spec.name);
        let parent_dir = packages_root_dir.join(&parent_dir_path);
        let lock_file_name = format!(".{}.lock", spec.version);
        let lock_file_path = parent_dir.join(&lock_file_name);
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir).map_err(other_err(
                "failed to create parent directories for lock file",
            ))?;
        }
        Ok(Self {
            file: fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&lock_file_path)
                .map_err(other_err("failed to open lock file"))?,
            path: lock_file_path,
            acquired_first_try: None,
            tmp_path: format!("{parent_dir_path}/{lock_file_name}"),
        })
    }

    /// Creates a `LockFile` and tries to acquire the exclusive lock.
    ///
    /// `packages_root_dir` is normally either the cache or the local directory.
    ///
    /// Whether the lock acquired on the first try or not can be checked
    /// with `did_wait()`. This can be useful for ... (do we need this sentence?)
    pub fn create_and_lock(
        packages_root_dir: &Path,
        spec: &PackageSpec,
    ) -> Result<Self, PackageError> {
        let mut lock_file = Self::new(packages_root_dir, spec)?;
        lock_file.acquired_first_try = Some(lock_file.try_and_lock()?); // something to tweak here?
        Ok(lock_file)
    }

    pub fn did_wait(&self) -> bool {
        self.acquired_first_try
            .expect("You must use it only after calling creat_and_lock!")
    }

    /// Try acquiring exclusive lock.
    fn try_lock(&self) -> io::Result<()> {
        self.file.try_lock_exclusive()
    }

    /// Acquire exclusive lock.
    fn lock(&self) -> Result<(), PackageError> {
        self.file
            .lock_exclusive()
            .map_err(other_err("failed to acquire lock file"))
    }

    /// Try acquiring the exclusive lock, if failed print an error an wait
    /// for the lock with blocking.
    ///
    /// Returns `Ok(true)` if the lock was acquired on the first try,
    /// otherwise `Ok(false)`.
    fn try_and_lock(&self) -> Result<bool, PackageError> {
        match self.try_lock() {
            Ok(_) => {
                eprintln!("[tmp] acquired the lock file {:?}", self.tmp_path);
                Ok(false)
            }
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                eprintln!("[tmp] couldn't acquire the lock file {:?}", self.tmp_path);
                eprintln!(
                    "Waiting for another instance to finish installing the package..."
                );
                self.lock()?;

                // // If other instance successfully installed a package after
                // // waiting for the lock, then there is nothing left to do.
                // if dir.exists() {
                //     eprintln!("[tmp] dropped the lock file {:?}", self.tmp_path);
                //     return Ok(dir);
                // }

                // eprintln!(
                //     "Another instance failed to install the package, trying it again."
                // );
                Ok(true)
                // Additional cool/user-friendly logs:
                // let _write_lock = lock_file.write().unwrap();
                // println!("checking if package already exists");
                // if dir.exists() {
                //     println!("it does! no need to download and unpack");
                //     return Ok(dir);
                // } else {
                //     println!("waited to acquire the file lock and the other instance wasn't able to download it :/");
                //     todo!()
                // }
            }
            Err(err) => Err(other_err("failed to acquire lock file")(err)),
        }
    }

    /// Remove (delete) lock file (if wasn't already by other process).
    ///
    /// Will be automatically called when struct is being dropped ([Drop]).
    fn remove(&self) -> Result<(), PackageError> {
        if self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(other_err("failed to remove lock file"))?;
        }
        eprintln!("[tmp] dropped the lock file {:?}", self.tmp_path);
        Ok(())
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        if let Err(error) = self.remove() {
            eprintln!("An error occured when dropping a lock file: {error}")
        }
    }
}
