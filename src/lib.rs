mod walker {
    use regex::Regex;
    use std::collections::VecDeque;
    use std::fs;
    use std::fs::{Metadata, ReadDir};
    use std::path::PathBuf;

    pub struct FileWalker {
        files: VecDeque<PathBuf>,
        dirs: VecDeque<PathBuf>,
        origin: PathBuf,
        max_depth: u32,
        follow_symlinks: bool,
        pattern: Option<Regex>,
    }

    impl FileWalker {
        pub fn for_path(
            path: &PathBuf,
            max_depth: u32,
            follow_symlinks: bool,
            pattern: Option<Regex>,
        ) -> FileWalker {
            let (files, dirs) =
                FileWalker::load(path, follow_symlinks).expect("Unable to load path");
            let dirs = if max_depth > 0 {
                VecDeque::from(dirs)
            } else {
                VecDeque::with_capacity(0)
            };
            let files = VecDeque::from(files);
            FileWalker {
                files,
                dirs,
                origin: path.clone(),
                max_depth,
                follow_symlinks,
                pattern,
            }
        }
        fn load(
            path: &PathBuf,
            follow_symlinks: bool,
        ) -> Result<(Vec<PathBuf>, Vec<PathBuf>), std::io::Error> {
            let path: ReadDir = read_dirs(&path)?;
            let (files, dirs) = path
                .filter_map(|p| p.ok())
                .map(|p| p.path())
                .filter(|p: &PathBuf| !is_symlink(p) || follow_symlinks)
                .filter(|p: &PathBuf| is_valid_target(p))
                .partition(|p| p.is_file());
            Ok((files, dirs))
        }
        fn push(&mut self, path: &PathBuf) {
            match FileWalker::load(path, self.follow_symlinks) {
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
            let comps0 = self.origin.canonicalize().unwrap().components().count();
            let comps1 = dir.canonicalize().unwrap().components().count();
            comps1 - comps0
        }
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
}

#[cfg(test)]
mod tests {
    use crate::walker::FileWalker;
    use std::path::PathBuf;

    const TEST_DIR: &str = "test_dirs";

    #[test]
    fn test_depth_only_root_dir() {
        let dir = PathBuf::from(TEST_DIR);
        let found = FileWalker::for_path(&dir, 0, false, None).count();
        assert_eq!(1, found);
    }

    #[test]
    fn test_depth_one() {
        let dir = PathBuf::from(TEST_DIR);
        let found = FileWalker::for_path(&dir, 1, false, None).count();
        assert_eq!(3, found);
    }
}
