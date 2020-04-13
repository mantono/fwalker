use std::path::PathBuf;
use sysinfo::DiskExt;
use sysinfo::{RefreshKind, System, SystemExt};

/// Given an array of known file systems and a path
pub(crate) fn fs_boundaries(filesystems: &[PathBuf], path: &PathBuf) -> Vec<PathBuf> {
    filesystems
        .iter()
        .filter(|fs| fs.starts_with(path) && *fs != path)
        .map(PathBuf::from)
        .collect()
}

pub(crate) fn filesystems() -> Vec<PathBuf> {
    let refresh: RefreshKind = RefreshKind::new().with_disks_list().with_disks();
    System::new_with_specifics(refresh)
        .get_disks()
        .iter()
        .map(|disk| disk.get_mount_point().to_path_buf())
        .collect::<Vec<PathBuf>>()
}

#[cfg(test)]
mod tests {
    use crate::fs::{filesystems, fs_boundaries};
    use std::path::PathBuf;

    #[test]
    fn test_filesystems_are_never_empty() {
        let filesystems: Vec<PathBuf> = filesystems();
        assert!(!filesystems.is_empty())
    }

    #[test]
    fn test_fs_boundaries_no_boundary() {
        let filesystems: Vec<PathBuf> = vec![
            "/proc",
            "/sys",
            "/sys/firmware/efi/efivars",
            "/dev",
            "/run",
            "/",
            "/tmp",
            "/home",
            "/boot",
            "/sys/kernel/security",
            "/sys/fs/cgroup/memory",
            "/sys/fs/cgroup/cpu,cpuacct",
            "/sys/fs/cgroup/freezer",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        let path: PathBuf = PathBuf::from("/home/user");
        let boundaries: Vec<PathBuf> = fs_boundaries(&filesystems, &path);
        assert_eq!(true, boundaries.is_empty())
    }

    #[test]
    fn test_fs_boundaries_single_boundary() {
        let filesystems: Vec<PathBuf> = vec![
            "/proc",
            "/sys",
            "/sys/firmware/efi/efivars",
            "/dev",
            "/run",
            "/",
            "/tmp",
            "/home",
            "/boot",
            "/sys/kernel/security",
            "/sys/fs/cgroup/memory",
            "/sys/fs/cgroup/cpu,cpuacct",
            "/sys/fs/cgroup/freezer",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        let path = PathBuf::from("/sys/kernel");
        let boundaries: Vec<PathBuf> = fs_boundaries(&filesystems, &path);
        let expected = vec![PathBuf::from("/sys/kernel/security")];
        assert_eq!(expected, boundaries)
    }

    #[test]
    fn test_fs_boundaries_do_not_include_file_system_in_boundaries() {
        let filesystems: Vec<PathBuf> = vec![
            "/proc",
            "/sys",
            "/sys/firmware/efi/efivars",
            "/dev",
            "/run",
            "/",
            "/tmp",
            "/home",
            "/boot",
            "/sys/kernel/security",
            "/sys/fs/cgroup/memory",
            "/sys/fs/cgroup/cpu,cpuacct",
            "/sys/fs/cgroup/freezer",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        let path = PathBuf::from("/");
        let boundaries: Vec<PathBuf> = fs_boundaries(&filesystems, &path);

        let expected: Vec<PathBuf> = vec![
            "/proc",
            "/sys",
            "/sys/firmware/efi/efivars",
            "/dev",
            "/run",
            "/tmp",
            "/home",
            "/boot",
            "/sys/kernel/security",
            "/sys/fs/cgroup/memory",
            "/sys/fs/cgroup/cpu,cpuacct",
            "/sys/fs/cgroup/freezer",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        assert_eq!(expected, boundaries)
    }

    #[test]
    fn test_fs_boundaries_multiple_boundaries() {
        let filesystems: Vec<PathBuf> = vec![
            "/proc",
            "/sys",
            "/sys/firmware/efi/efivars",
            "/dev",
            "/run",
            "/",
            "/tmp",
            "/home",
            "/boot",
            "/sys/kernel/security",
            "/sys/fs/cgroup/memory",
            "/sys/fs/cgroup/cpu,cpuacct",
            "/sys/fs/cgroup/freezer",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();

        let path: PathBuf = PathBuf::from("/sys/fs");
        let boundaries: Vec<PathBuf> = fs_boundaries(&filesystems, &path);
        let expected = vec![
            PathBuf::from("/sys/fs/cgroup/memory"),
            PathBuf::from("/sys/fs/cgroup/cpu,cpuacct"),
            PathBuf::from("/sys/fs/cgroup/freezer"),
        ];
        assert_eq!(expected, boundaries)
    }
}
