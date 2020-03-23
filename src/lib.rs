use std::collections::VecDeque;
use std::fs;
use std::fs::{Metadata, ReadDir};
use std::io::ErrorKind;
use std::path::PathBuf;

#[derive(Default, Clone)]
pub struct FileWalker {
    files: VecDeque<PathBuf>,
    dirs: VecDeque<PathBuf>,
    origin: PathBuf,
    max_depth: u32,
    follow_symlinks: bool,
}

impl FileWalker {
    /// Create a new FileWalker starting from the current directory (path `.`).
    /// This FileWalker will not follow symlinks and will not have any limitation
    /// in recursion depth for directories.
    pub fn new() -> Result<FileWalker, std::io::Error> {
        FileWalker::for_path(&PathBuf::from("."), std::u32::MAX, false)
    }

    /// Create a new FileWalker for the given path, while also specifying the
    /// max recursion depth and if symlinks should be followed or not.
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
    /// let max_depth: u32 = 100;
    /// let follow_symlinks: bool = false;
    /// let mut walker = walker::FileWalker::for_path(&path, max_depth, follow_symlinks)?;
    ///
    /// assert_eq!(Some(PathBuf::from("test_dirs/file0").canonicalize()?), walker.next());
    /// assert_eq!(Some(PathBuf::from("test_dirs/sub_dir/file2").canonicalize()?), walker.next());
    /// assert_eq!(Some(PathBuf::from("test_dirs/sub_dir/file1").canonicalize()?), walker.next());
    /// assert_eq!(None, walker.next());
    /// #
    /// #    Ok(())
    /// # }
    /// ```
    pub fn for_path(
        path: &PathBuf,
        max_depth: u32,
        follow_symlinks: bool,
    ) -> Result<FileWalker, std::io::Error> {
        if !path.exists() {
            let err = std::io::Error::from(ErrorKind::NotFound);
            return Err(err)
        }
        if !path.is_dir() {
            let err = std::io::Error::new(ErrorKind::InvalidInput, "Path is not a directory");
            return Err(err)
        }
        let mut dirs = VecDeque::with_capacity(1);
        dirs.push_back(path.clone());
        let files = VecDeque::with_capacity(0);

        let walker = FileWalker {
            files,
            dirs,
            origin: path.clone(),
            max_depth,
            follow_symlinks,
        };
        Ok(walker)
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
        let comps0 = components(&self.origin);
        let comps1 = components(dir);
        comps1 - comps0
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

#[cfg(test)]
mod tests {
    use crate::FileWalker;
    use std::path::PathBuf;

    const TEST_DIR: &str = "test_dirs";

    #[test]
    fn test_depth_only_root_dir() {
        let dir = PathBuf::from(TEST_DIR);
        let found = FileWalker::for_path(&dir, 0, false).unwrap().count();
        assert_eq!(1, found);
    }

    #[test]
    fn test_depth_one() {
        let dir = PathBuf::from(TEST_DIR);
        let found = FileWalker::for_path(&dir, 1, false).unwrap().count();
        assert_eq!(3, found);
    }

    #[test]
    fn test_path_not_found() {
        let dir = PathBuf::from("/dev/null/foo");
        match FileWalker::for_path(&dir, 1, false) {
            Err(error) => assert_eq!(std::io::ErrorKind::NotFound, error.kind()),
            _ => panic!()
        }
    }

    #[test]
    fn test_path_not_a_dir() {
        let dir = PathBuf::from("src/lib.rs");
        match FileWalker::for_path(&dir, 1, false) {
            Err(error) => assert_eq!(std::io::ErrorKind::InvalidInput, error.kind()),
            _ => panic!()
        }
    }
}
