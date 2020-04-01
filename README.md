# fwalker:walking:
A cargo crate for file and directory traversal in a file system through an iterator

![Build & Test](https://github.com/mantono/fwalker/workflows/Build%20&%20Test/badge.svg?branch=master)
![Security Audit](https://github.com/mantono/fwalker/workflows/Security%20Audit/badge.svg)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Documentation
See [docs.rs/fwalker](https://docs.rs/fwalker/) for complete documentation

## Usage

This crate has only one public struct, `fwalker::Walker`. With this struct, files and directories
can be iterated over for any kind of listing, manipulation or processing. Creating a new walker can
be done with either
- `fwalker::Walker::new()` - starts from current directory
- `fwalker::Walker::from("/some/path")` - starts from path `/some/path`

Quick example to get you started
```rust
use fwalker::Walker;
use std::path::PathBuf;

fn main() {
    Walker::from("/proc/sys")
        .expect("This *should* work")
        .take(10)
        .for_each(|file: PathBuf| println!("{:?}", file));
}
```

which would yield the output

```
"/proc/sys/abi/vsyscall32"
"/proc/sys/debug/exception-trace"
"/proc/sys/debug/kprobes-optimization"
"/proc/sys/fs/aio-max-nr"
"/proc/sys/fs/aio-nr"
"/proc/sys/fs/dentry-state"
"/proc/sys/fs/dir-notify-enable"
"/proc/sys/fs/file-max"
"/proc/sys/fs/file-nr"
"/proc/sys/fs/inode-nr"
```