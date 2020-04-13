use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::fs::{Metadata, ReadDir};
use std::io::ErrorKind;
use std::path::PathBuf;

mod fs;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Walker {
    files: VecDeque<PathBuf>,
    dirs: VecDeque<PathBuf>,
    ignore: Vec<PathBuf>,
    origin: PathBuf,
    origin_depth: usize,
    max_depth: u32,
    follow_symlinks: bool,
}

impl Walker {
    /// Create a new Walker starting from the current directory (path `.`).
    /// This Walker will not follow symlinks and will not have any limitation
    /// in recursion depth for directories.
    pub fn new() -> Result<Walker, std::io::Error> {
        Walker::from(&PathBuf::from("."))
    }

    /// Create a new Walker for the given path. This Walker will not follow
    /// symlinks and will not have any limitation in recursion depth for directories.
    ///
    /// With a directory structure of
    ///
    /// ```text
    /// file0
    /// dir0/
    /// ├── file1
    /// ├── file2
    /// ├── empty_dir/
    /// ├── .hidden_dir/
    /// │   └── file3
    /// └── .hidden_file
    /// ```
    ///
    /// the Walker should return the files as following
    /// ```
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::path::PathBuf;
    ///
    /// let walker = fwalker::Walker::from("test_dirs")?;
    /// let found_files: usize = walker.count();
    /// assert_eq!(5, found_files);
    /// #
    /// #    Ok(())
    /// # }
    ///
    /// ```
    /// Walker::from takes any argument that can be coverted into a PathBuf, so the following
    /// is possible as well
    /// ```
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let mut walker = fwalker::Walker::from("test_dirs")?;
    /// assert!(walker.next().is_some());
    /// #
    /// #    Ok(())
    /// # }
    /// ```
    pub fn from<T: Into<PathBuf>>(path: T) -> Result<Walker, std::io::Error> {
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

        let walker = Walker {
            files,
            dirs,
            ignore: vec![],
            origin: path.to_path_buf(),
            origin_depth: components(&path),
            max_depth: std::u32::MAX,
            follow_symlinks: false,
        };
        Ok(walker)
    }

    /// Modifies the current instance of a Walker, retaining the current configuration for the
    /// Walker, but setting the maximum recursion depth to the maximum value of `depth`.
    pub fn max_depth(mut self, depth: u32) -> Walker {
        self.max_depth = depth;
        self
    }

    /// Enable following of symlinks on the current Walker when traversing through files.
    /// Once this option has been enabled for a Walker, it cannot be disabled again.
    pub fn follow_symlinks(mut self) -> Walker {
        self.follow_symlinks = true;
        self
    }

    /// Reset a Walker to its original state, starting over with iterating from the _origin_
    /// `PathBuf`. Changes made to the Walker after it was created with `max_depth()` and
    /// `follow_symlinks()` will not be reseted.
    ///
    /// Unlike when the Walker was initially created, no validation will be done that the
    /// path actually exists or that it is a directory, since both of these conditions must have
    /// been met when the Walker was created.
    pub fn reset(&mut self) -> &mut Walker {
        self.files.clear();
        self.dirs.clear();
        self.dirs.push_back(self.origin.to_path_buf());
        self
    }

    /// Prevent the Walker from entering other file systems while traversing a directory structure.
    /// This means that subdirectories of a directory that belongs to another file system will be
    /// ignored.
    pub fn only_local_fs(mut self) -> Result<Walker, std::io::Error> {
        let filesystems = fs::filesystems();
        self.ignore = fs::fs_boundaries(&filesystems, &self.origin);
        Ok(self)
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
                    let dirs: Vec<PathBuf> = filter_boundaries(dirs, &self.ignore);
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
    path.canonicalize()
        .expect("Unable to canonicalize path")
        .components()
        .count()
}

fn filter_boundaries(dirs: Vec<PathBuf>, boundaries: &[PathBuf]) -> Vec<PathBuf> {
    dirs.iter()
        .filter(|d| !boundaries.contains(d))
        .map(|d| d.to_path_buf())
        .collect()
}

impl Iterator for Walker {
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
    Ok(std::fs::read_dir(full_path)?)
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

impl std::fmt::Display for Walker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "origin: {:?}, current file: {:?}, current directory: {:?}",
            &self.origin,
            self.files.get(0),
            self.dirs.get(0)
        )
    }
}

impl Default for Walker {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl std::cmp::Ord for Walker {
    fn cmp(&self, other: &Self) -> Ordering {
        let left: usize = current_depth(self);
        let right: usize = current_depth(other);
        right.cmp(&left)
    }
}

fn current_depth(walker: &Walker) -> usize {
    let fallback: PathBuf = PathBuf::new();
    let path: &PathBuf = walker
        .files
        .get(0)
        .unwrap_or_else(|| walker.dirs.get(0).unwrap_or(&fallback));
    components(path)
}

impl std::cmp::PartialOrd for Walker {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// All unit tests are run under the asumption that the `test_dirs`
/// directory has the following structure
///
/// ```text
/// file0
/// dir0/
/// ├── file1
/// ├── file2
/// ├── empty_dir/
/// ├── .hidden_dir/
/// │   └── file3
/// └── .hidden_file
/// ```
/// and consisting of five files in total.
#[cfg(test)]
mod tests {
    use crate::Walker;
    use std::cmp::Ordering;
    use std::path::PathBuf;

    const TEST_DIR: &str = "test_dirs";

    #[test]
    fn test_depth_only_root_dir() {
        let dir = PathBuf::from(TEST_DIR);
        let found = Walker::from(&dir).unwrap().max_depth(0).count();
        assert_eq!(1, found);
    }

    #[test]
    fn test_depth_one() {
        let dir = PathBuf::from(TEST_DIR);
        let found = Walker::from(&dir).unwrap().max_depth(1).count();
        assert_eq!(4, found);
    }

    #[test]
    fn test_reset() {
        let mut walker = Walker::from(TEST_DIR).unwrap();
        let file0: PathBuf = walker.next().unwrap();
        walker.reset();
        let file1: PathBuf = walker.next().unwrap();
        assert_eq!(file0, file1);
    }

    #[test]
    fn test_path_not_found() {
        let dir = PathBuf::from("/dev/null/foo");
        match Walker::from(&dir) {
            Err(error) => assert_eq!(std::io::ErrorKind::NotFound, error.kind()),
            _ => panic!(),
        }
    }

    #[test]
    fn test_path_not_a_dir() {
        let dir = PathBuf::from("src/lib.rs");
        match Walker::from(&dir) {
            Err(error) => assert_eq!(std::io::ErrorKind::InvalidInput, error.kind()),
            _ => panic!(),
        }
    }

    #[test]
    fn test_equals() {
        let walker0 = Walker::from(TEST_DIR).unwrap();
        let walker1 = Walker::from(TEST_DIR).unwrap();
        assert_eq!(walker0, walker1)
    }

    #[test]
    fn test_not_equals_different_origin() {
        let other_dir: String = format!("{}/dir0", TEST_DIR);
        let walker0 = Walker::from(TEST_DIR).unwrap();
        let walker1 = Walker::from(other_dir).unwrap();
        assert_ne!(walker0, walker1)
    }

    #[test]
    fn test_not_equals_different_state() {
        let walker0 = Walker::from(TEST_DIR).unwrap();
        let mut walker1 = Walker::from(TEST_DIR).unwrap();
        walker1.next();
        assert_ne!(walker0, walker1)
    }

    #[test]
    fn test_not_equals_different_settings() {
        let walker0: Walker = Walker::from(TEST_DIR).unwrap().max_depth(1);
        let walker1: Walker = Walker::from(TEST_DIR).unwrap().follow_symlinks();
        assert_ne!(walker0, walker1)
    }

    #[test]
    fn test_default() {
        let walker0: Walker = Walker::new().unwrap();
        let walker1: Walker = Default::default();
        assert_eq!(walker0, walker1)
    }

    #[test]
    fn test_ordering_less_than() {
        let mut walker0 = Walker::from(TEST_DIR).unwrap();
        let walker1 = Walker::from(TEST_DIR).unwrap();
        walker0.next();
        walker0.next();
        assert!(walker0 < walker1)
    }

    #[test]
    fn test_ordering_greater_than() {
        let walker0 = Walker::from(TEST_DIR).unwrap();
        let mut walker1 = Walker::from(TEST_DIR).unwrap();
        walker1.next();
        walker1.next();
        assert!(walker0 > walker1)
    }

    #[test]
    fn test_ordering_equal() {
        let walker0 = Walker::from(TEST_DIR).unwrap();
        let walker1 = Walker::from(TEST_DIR).unwrap();
        assert_eq!(walker0.cmp(walker1), Ordering::Equal)
    }
}
