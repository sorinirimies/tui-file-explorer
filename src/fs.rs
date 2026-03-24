//! Filesystem helpers for the `tfe` binary.
//!
//! This module contains small, pure filesystem utilities that have no
//! dependency on application state or terminal rendering:
//!
//! * [`copy_dir_all`]        — recursively copy a directory tree.
//! * [`resolve_output_path`] — apply the `--print-dir` flag to a selected path.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

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
}
