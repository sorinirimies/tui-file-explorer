//! Filesystem helpers for the `tfe` binary.
//!
//! This module contains small, pure filesystem utilities that have no
//! dependency on application state or terminal rendering:
//!
//! * [`copy_dir_all`]        — recursively copy a directory tree.
//! * [`resolve_output_path`] — apply the `--print-dir` flag to a selected path.
//! * [`dir_size`]            — bounded recursive directory byte-size.
//! * [`disk_usage`]          — total / free space for the backing storage device.

use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::types::DiskUsage;

// ── Directory copy ────────────────────────────────────────────────────────────

/// Recursively copy the directory tree rooted at `src` to `dst`.
///
/// `dst` and any missing parent directories are created automatically.
/// Existing files inside `dst` are silently overwritten. Symlinks are not
/// followed — only regular files and directories are processed.
///
/// # Errors
///
/// Returns an [`io::Error`] if any read, create, or copy operation fails.
///
/// # Example
///
/// ```no_run
/// # use std::path::Path;
/// # use std::fs;
/// # let src = Path::new("/tmp/src");
/// # let dst = Path::new("/tmp/dst");
/// // Copy a directory tree — dst is created automatically.
/// // copy_dir_all(src, dst)?;
/// ```
pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)?.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        // Skip symlinks — they would require platform-specific handling and
        // are common in build artefact directories (e.g. Android `build/`).
        if src_path.is_symlink() {
            continue;
        }
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// ── Path output ───────────────────────────────────────────────────────────────

/// Resolve the output path from a selected path and the `--print-dir` flag.
///
/// When `print_dir` is `true` the parent directory of `path` is returned,
/// falling back to `path` itself if there is no parent (e.g. filesystem root).
/// When `print_dir` is `false` the original `path` is returned unchanged.
pub fn resolve_output_path(path: PathBuf, print_dir: bool) -> PathBuf {
    if print_dir {
        path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
    } else {
        path
    }
}

// ── Disk usage ────────────────────────────────────────────────────────────────

/// Query total and free space (in bytes) for the storage device backing
/// `path`.
///
/// `path` may be a file or a directory; the query always targets the
/// filesystem/mount point that contains it. Returns `None` if the OS call
/// fails (e.g. the path does not exist, or the platform is unsupported).
///
/// This intentionally reports the *device* total/free space, not directory
/// content size — that is a distinct, much cheaper-to-compute value exposed
/// per-entry via [`crate::FsEntry::size`] (files) and
/// [`crate::FsEntry::item_count`] (directories).
#[cfg(unix)]
pub fn disk_usage(path: &Path) -> Option<DiskUsage> {
    use std::os::unix::ffi::OsStrExt;

    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;

    // SAFETY: `stat` is plain-old-data with no invalid bit patterns; we only
    // read its fields after `statvfs` returns success (0), at which point
    // the OS has fully initialised them.
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) != 0 {
            return None;
        }
        let block_size = stat.f_frsize as u64;
        let total_bytes = block_size.saturating_mul(stat.f_blocks as u64);
        // f_bavail = blocks available to the calling (unprivileged) process,
        // matching what `df` reports as "available".
        let free_bytes = block_size.saturating_mul(stat.f_bavail as u64);
        Some(DiskUsage {
            total_bytes,
            free_bytes,
        })
    }
}

/// Windows implementation using `GetDiskFreeSpaceExW`.
#[cfg(windows)]
pub fn disk_usage(path: &Path) -> Option<DiskUsage> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    // The API expects a directory (or drive root); fall back to the parent
    // directory when `path` points at a file.
    let dir: &Path = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };

    let wide: Vec<u16> = dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_available_bytes: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free_bytes: u64 = 0;

    // SAFETY: `wide` is a valid, NUL-terminated UTF-16 string for the
    // lifetime of the call; the three out-parameters are valid `u64` slots.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_available_bytes,
            &mut total_bytes,
            &mut total_free_bytes,
        )
    };

    if ok == 0 {
        None
    } else {
        Some(DiskUsage {
            total_bytes,
            free_bytes: free_available_bytes,
        })
    }
}

/// Fallback for platforms with no supported disk-usage query.
#[cfg(not(any(unix, windows)))]
pub fn disk_usage(_path: &Path) -> Option<DiskUsage> {
    None
}

// ── Directory size ─────────────────────────────────────────────────────────

/// Maximum number of filesystem entries [`dir_size`] visits before giving up
/// and returning a partial (lower-bound) total.
///
/// Keeps the recursive walk bounded so browsing a directory that contains a
/// huge subtree (`node_modules`, `.git`, `target`, a whole external drive...)
/// can never freeze the UI for more than a fraction of a second.
pub const DIR_SIZE_MAX_ENTRIES: usize = 20_000;

/// Maximum wall-clock time [`dir_size`] spends walking before giving up.
///
/// A secondary safety net alongside [`DIR_SIZE_MAX_ENTRIES`] for slow
/// filesystems (network mounts, spun-down external drives, ...) where even a
/// modest entry count can take a long time to stat.
pub const DIR_SIZE_MAX_DURATION: Duration = Duration::from_millis(25);

/// Recursively sum the size of every regular file under `dir`.
///
/// Returns `(total_bytes, is_partial)`. `is_partial` is `true` when the walk
/// was cut short by [`DIR_SIZE_MAX_ENTRIES`] or [`DIR_SIZE_MAX_DURATION`], in
/// which case `total_bytes` is a lower bound rather than the exact size.
///
/// Symlinks are not followed (their target is never counted), which also
/// sidesteps infinite loops from cyclic links. Entries that can't be read
/// (permission denied, removed mid-walk, ...) are silently skipped.
///
/// The walk is iterative (an explicit stack) rather than recursive, so a
/// pathologically deep directory tree can't blow the call stack.
pub fn dir_size(dir: &Path) -> (u64, bool) {
    let start = Instant::now();
    let mut total: u64 = 0;
    let mut visited: usize = 0;
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(read) = fs::read_dir(&current) else {
            continue;
        };

        for entry in read.flatten() {
            if visited >= DIR_SIZE_MAX_ENTRIES || start.elapsed() >= DIR_SIZE_MAX_DURATION {
                return (total, true);
            }
            visited += 1;

            let path = entry.path();
            if path.is_symlink() {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }

    (total, false)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // ── copy_dir_all ──────────────────────────────────────────────────────────

    #[test]
    fn copy_dir_all_copies_single_file() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("src");
        fs::create_dir(&src).expect("mkdir src");
        fs::write(src.join("file.txt"), b"hello").expect("write");

        let dst = dir.path().join("dst");
        copy_dir_all(&src, &dst).expect("copy_dir_all");

        assert!(dst.join("file.txt").exists());
        assert_eq!(fs::read(dst.join("file.txt")).expect("read"), b"hello");
    }

    #[test]
    fn copy_dir_all_copies_nested_structure() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("src");
        let sub = src.join("sub");
        fs::create_dir_all(&sub).expect("mkdir sub");
        fs::write(src.join("a.txt"), b"a").expect("write a");
        fs::write(sub.join("b.txt"), b"b").expect("write b");

        let dst = dir.path().join("dst");
        copy_dir_all(&src, &dst).expect("copy_dir_all");

        assert!(dst.join("a.txt").exists());
        assert!(dst.join("sub").join("b.txt").exists());
        assert_eq!(fs::read(dst.join("sub").join("b.txt")).expect("read"), b"b");
    }

    #[test]
    fn copy_dir_all_creates_dst_when_absent() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("src");
        fs::create_dir(&src).expect("mkdir src");
        fs::write(src.join("x.txt"), b"x").expect("write");

        let dst = dir.path().join("deep/nested/dst");
        copy_dir_all(&src, &dst).expect("copy_dir_all should create missing parents");

        assert!(dst.join("x.txt").exists());
    }

    #[test]
    fn copy_dir_all_overwrites_existing_file_in_dst() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        fs::create_dir_all(&src).expect("mkdir src");
        fs::create_dir_all(&dst).expect("mkdir dst");
        fs::write(src.join("f.txt"), b"new").expect("write src");
        fs::write(dst.join("f.txt"), b"old").expect("write dst");

        copy_dir_all(&src, &dst).expect("copy_dir_all");

        assert_eq!(fs::read(dst.join("f.txt")).expect("read"), b"new");
    }

    #[test]
    fn copy_dir_all_empty_src_creates_empty_dst() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("src");
        fs::create_dir(&src).expect("mkdir src");

        let dst = dir.path().join("dst");
        copy_dir_all(&src, &dst).expect("copy_dir_all");

        assert!(dst.exists());
        assert_eq!(
            fs::read_dir(&dst).expect("read_dir").count(),
            0,
            "dst should be empty"
        );
    }

    #[test]
    fn copy_dir_all_leaves_source_intact() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("src");
        fs::create_dir(&src).expect("mkdir src");
        fs::write(src.join("keep.txt"), b"original").expect("write");

        let dst = dir.path().join("dst");
        copy_dir_all(&src, &dst).expect("copy_dir_all");

        assert!(src.join("keep.txt").exists(), "source must survive a copy");
    }

    #[test]
    fn copy_dir_all_nonexistent_src_returns_error() {
        let dir = tempdir().expect("tempdir");
        let src = dir.path().join("does_not_exist");
        let dst = dir.path().join("dst");

        let result = copy_dir_all(&src, &dst);
        assert!(result.is_err(), "expected an error for missing src");
    }

    // ── resolve_output_path ───────────────────────────────────────────────────

    #[test]
    fn resolve_output_path_print_dir_false_returns_original() {
        let path = PathBuf::from("/some/dir/file.txt");
        let result = resolve_output_path(path.clone(), false);
        assert_eq!(result, path);
    }

    #[test]
    fn resolve_output_path_print_dir_true_returns_parent() {
        let path = PathBuf::from("/some/dir/file.txt");
        let result = resolve_output_path(path, true);
        assert_eq!(result, PathBuf::from("/some/dir"));
    }

    #[test]
    fn resolve_output_path_print_dir_true_at_root_returns_root() {
        // On Unix "/" has no parent — should fall back to the path itself.
        let path = PathBuf::from("/");
        let result = resolve_output_path(path.clone(), true);
        assert_eq!(result, path);
    }

    #[test]
    fn resolve_output_path_dir_path_returns_parent_dir() {
        let path = PathBuf::from("/home/user/projects");
        let result = resolve_output_path(path, true);
        assert_eq!(result, PathBuf::from("/home/user"));
    }

    // ── disk_usage ───────────────────────────────────────────────────────

    #[test]
    fn disk_usage_returns_some_for_existing_dir() {
        let dir = tempdir().expect("tempdir");
        let usage = disk_usage(dir.path());
        assert!(usage.is_some(), "disk_usage should succeed for a real path");
        let usage = usage.unwrap();
        assert!(usage.total_bytes > 0, "total_bytes should be non-zero");
        assert!(
            usage.free_bytes <= usage.total_bytes,
            "free space can't exceed total space"
        );
    }

    #[test]
    fn disk_usage_works_for_a_file_path_too() {
        let dir = tempdir().expect("tempdir");
        let file = dir.path().join("f.txt");
        fs::write(&file, b"x").expect("write");
        let usage = disk_usage(&file);
        assert!(usage.is_some(), "disk_usage should accept a file path");
    }

    // ── dir_size ──────────────────────────────────────────────────────────

    #[test]
    fn dir_size_sums_files_in_flat_directory() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("a.txt"), vec![0u8; 100]).unwrap();
        fs::write(dir.path().join("b.txt"), vec![0u8; 250]).unwrap();

        let (total, partial) = dir_size(dir.path());
        assert_eq!(total, 350);
        assert!(!partial);
    }

    #[test]
    fn dir_size_recurses_into_subdirectories() {
        let dir = tempdir().expect("tempdir");
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(dir.path().join("top.txt"), vec![0u8; 10]).unwrap();
        fs::write(sub.join("nested.txt"), vec![0u8; 20]).unwrap();

        let (total, partial) = dir_size(dir.path());
        assert_eq!(total, 30);
        assert!(!partial);
    }

    #[test]
    fn dir_size_empty_directory_is_zero() {
        let dir = tempdir().expect("tempdir");
        let (total, partial) = dir_size(dir.path());
        assert_eq!(total, 0);
        assert!(!partial);
    }

    #[test]
    fn dir_size_nonexistent_directory_returns_zero() {
        let dir = tempdir().expect("tempdir");
        let missing = dir.path().join("nope");
        let (total, partial) = dir_size(&missing);
        assert_eq!(total, 0);
        assert!(!partial);
    }
}
