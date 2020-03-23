use std::path::PathBuf;
use std::fmt;

#[derive(Debug, Clone)]
pub struct FileError {
    pub path: PathBuf,
    pub message: String,
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}: {}", self.path, self.message)
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self)
    }
}