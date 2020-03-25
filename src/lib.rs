use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::fs;
use std::fs::{Metadata, ReadDir};
use std::io::ErrorKind;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FileWalker {
    files: VecDeque<PathBuf>,
    dirs: VecDeque<PathBuf>,
    origin_depth: usize,
    max_depth: u32,
    follow_symlinks: bool,
}

impl FileWalker {
    /// Create a new FileWalker starting from the current directory (path `.`).
    /// This FileWalker will not follow symlinks and will not have any limitation
    /// in recursion depth for directories.
    pub fn new() -> Result<FileWalker, std::io::Error> {
        FileWalker::from(&PathBuf::from("."))
    }

    /// Create a new FileWalker for the given path. This FileWalker will not follow
    /// symlinks and will not have any limitation in recursion depth for directories.
    ///
    /// With a directory structure of
    ///
    /// ```yaml
    /// test_dirs:
    ///   - file0
    ///   sub_dir:
    ///     - file1
    ///     - file2
    /// ```
    ///
    /// the FileWalker should return the files as following
    /// ```
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::path::PathBuf;
    ///
    /// let path = PathBuf::from("test_dirs");
    /// let mut walker = walker::FileWalker::from(&path)?;
    ///
    /// assert_eq!(Some(PathBuf::from("test_dirs/file0").canonicalize()?), walker.next());
    /// assert_eq!(Some(PathBuf::from("test_dirs/sub_dir/file2").canonicalize()?), walker.next());
    /// assert_eq!(Some(PathBuf::from("test_dirs/sub_dir/file1").canonicalize()?), walker.next());
    /// assert_eq!(None, walker.next());
    /// #
    /// #    Ok(())
    /// # }
    ///
    /// ```
    /// FileWalker::from takes any argument that can be coverted into a PathBuf, so the following
    /// is possible as well
    /// ```
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let mut walker = walker::FileWalker::from("test_dirs")?;
    /// assert!(walker.next().is_some());
    /// #
    /// #    Ok(())
    /// # }
    /// ```
    pub fn from<T: Into<PathBuf>>(path: T) -> Result<FileWalker, std::io::Error> {
        let path: &PathBuf = &path.into();
        if !path.exists() {
            let err = std::io::Error::from(ErrorKind::NotFound);
            return Err(err);
        }
        if !path.is_dir() {
            let err = std::io::Error::new(ErrorKind::InvalidInput, "Path is not a directory");
            return Err(err);
        }
        let mut dirs = VecDeque::with_capacity(1);
        dirs.push_back(path.clone());
        let files = VecDeque::with_capacity(0);

        let walker = FileWalker {
            files,
            dirs,
            origin_depth: components(&path),
            max_depth: std::u32::MAX,
            follow_symlinks: false,
        };
        Ok(walker)
    }

    /// Modifies the current instance of a FileWalker, retaining the current configuration for the
    /// FileWalker, but setting the maximum recursion depth to the maximum value of `depth`.
    pub fn max_depth(mut self, depth: u32) -> FileWalker {
        self.max_depth = depth;
        self
    }

    /// Enable following of symlinks on the current FileWalker when traversing through files.
    /// Once this option has been enabled for a FileWalker, it cannot be disabled again.
    pub fn follow_symlinks(mut self) -> FileWalker {
        self.follow_symlinks = true;
        self
    }

    fn load(&self, path: &PathBuf) -> Result<(Vec<PathBuf>, Vec<PathBuf>), std::io::Error> {
        let path: ReadDir = read_dirs(&path)?;
        let (files, dirs) = path
            .filter_map(|p| p.ok())
            .map(|p| p.path())
            .filter(|p: &PathBuf| self.follow_symlinks || !is_symlink(p))
            .filter(is_valid_target)
            .partition(|p| p.is_file());
        Ok((files, dirs))
    }
    fn push(&mut self, path: &PathBuf) {
        match self.load(path) {
            Ok((files, dirs)) => {
                self.files.extend(files);
                let current_depth: u32 = self.depth(path) as u32;
                if current_depth < self.max_depth {
                    self.dirs.extend(dirs);
                }
            }
            Err(e) => log::warn!("{}: {:?}", e, path),
        }
    }
    fn depth(&self, dir: &PathBuf) -> usize {
        components(dir) - self.origin_depth
    }
}

fn components(path: &PathBuf) -> usize {
    path.canonicalize().expect("Unable to canonicalize path").components().count()
}

impl Iterator for FileWalker {
    type Item = PathBuf;
    fn next(&mut self) -> Option<Self::Item> {
        match self.files.pop_front() {
            Some(f) => Some(f),
            None => match self.dirs.pop_front() {
                Some(d) => {
                    self.push(&d);
                    self.next()
                }
                None => None,
            },
        }
    }
}

fn read_dirs(path: &PathBuf) -> Result<ReadDir, std::io::Error> {
    let full_path: PathBuf = path.canonicalize()?;
    Ok(fs::read_dir(full_path)?)
}

fn is_valid_target(path: &PathBuf) -> bool {
    let metadata: Metadata = path.metadata().expect("Unable to retrieve metadata:");
    metadata.is_file() || metadata.is_dir()
}

fn is_symlink(path: &PathBuf) -> bool {
    match path.symlink_metadata() {
        Ok(sym) => sym.file_type().is_symlink(),
        Err(err) => {
            log::warn!("{}: {:?}", err, path);
            false
        }
    }
}

impl std::fmt::Display for FileWalker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "current file: {:?}, current directory: {:?}",
            self.files.get(0),
            self.dirs.get(0)
        )
    }
}

impl Default for FileWalker {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl std::cmp::Ord for FileWalker {
    fn cmp(&self, other: &Self) -> Ordering {
        let left: usize = current_depth(self);
        let right: usize = current_depth(other);
        right.cmp(&left)
    }
}

fn current_depth(walker: &FileWalker) -> usize {
    let fallback: PathBuf = PathBuf::new();
    let path: &PathBuf =
        walker.files.get(0).unwrap_or_else(|| walker.dirs.get(0).unwrap_or(&fallback));
    components(path)
}

impl std::cmp::PartialOrd for FileWalker {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use crate::FileWalker;
    use std::cmp::Ordering;
    use std::path::PathBuf;

    const TEST_DIR: &str = "test_dirs";

    #[test]
    fn test_depth_only_root_dir() {
        let dir = PathBuf::from(TEST_DIR);
        let found = FileWalker::from(&dir).unwrap().max_depth(0).count();
        assert_eq!(1, found);
    }

    #[test]
    fn test_depth_one() {
        let dir = PathBuf::from(TEST_DIR);
        let found = FileWalker::from(&dir).unwrap().max_depth(1).count();
        assert_eq!(3, found);
    }

    #[test]
    fn test_path_not_found() {
        let dir = PathBuf::from("/dev/null/foo");
        match FileWalker::from(&dir) {
            Err(error) => assert_eq!(std::io::ErrorKind::NotFound, error.kind()),
            _ => panic!(),
        }
    }

    #[test]
    fn test_path_not_a_dir() {
        let dir = PathBuf::from("src/lib.rs");
        match FileWalker::from(&dir) {
            Err(error) => assert_eq!(std::io::ErrorKind::InvalidInput, error.kind()),
            _ => panic!(),
        }
    }

    #[test]
    fn test_equals() {
        let walker0 = FileWalker::from(TEST_DIR).unwrap();
        let walker1 = FileWalker::from(TEST_DIR).unwrap();
        assert_eq!(walker0, walker1)
    }

    #[test]
    fn test_not_equals_different_origin() {
        let other_dir: String = format!("{}/sub_dir", TEST_DIR);
        let walker0 = FileWalker::from(TEST_DIR).unwrap();
        let walker1 = FileWalker::from(other_dir).unwrap();
        assert_ne!(walker0, walker1)
    }

    #[test]
    fn test_not_equals_different_state() {
        let walker0 = FileWalker::from(TEST_DIR).unwrap();
        let mut walker1 = FileWalker::from(TEST_DIR).unwrap();
        walker1.next();
        assert_ne!(walker0, walker1)
    }

    #[test]
    fn test_not_equals_different_settings() {
        let walker0: FileWalker = FileWalker::from(TEST_DIR).unwrap().max_depth(1);
        let walker1: FileWalker = FileWalker::from(TEST_DIR).unwrap().follow_symlinks();
        assert_ne!(walker0, walker1)
    }

    #[test]
    fn test_default() {
        let walker0: FileWalker = FileWalker::new().unwrap();
        let walker1: FileWalker = Default::default();
        assert_eq!(walker0, walker1)
    }

    #[test]
    fn test_ordering_less_than() {
        let mut walker0 = FileWalker::from(TEST_DIR).unwrap();
        let walker1 = FileWalker::from(TEST_DIR).unwrap();
        walker0.next();
        walker0.next();
        assert!(walker0 < walker1)
    }

    #[test]
    fn test_ordering_greater_than() {
        let walker0 = FileWalker::from(TEST_DIR).unwrap();
        let mut walker1 = FileWalker::from(TEST_DIR).unwrap();
        walker1.next();
        walker1.next();
        assert!(walker0 > walker1)
    }

    #[test]
    fn test_ordering_equal() {
        let walker0 = FileWalker::from(TEST_DIR).unwrap();
        let walker1 = FileWalker::from(TEST_DIR).unwrap();
        assert_eq!(walker0.cmp(walker1), Ordering::Equal)
    }
}
