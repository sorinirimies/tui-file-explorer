//! Lightweight `git status` integration: a per-entry change marker.
//!
//! Shells out to the `git` binary (no libgit2 dependency). Everything here is
//! best-effort: no repository, no `git` on `PATH`, or any failure simply
//! yields no markers.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

/// Change state of a path, ordered by display priority (highest last).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GitMark {
    /// Untracked file (`?`).
    Untracked,
    /// Added / staged new file (`A`).
    Added,
    /// Renamed or copied (`R`).
    Renamed,
    /// Deleted (`D`).
    Deleted,
    /// Modified in the index or work tree (`M`).
    Modified,
    /// Unmerged / conflicted (`U`).
    Conflict,
}

impl GitMark {
    /// Single-character marker.
    #[must_use]
    pub fn symbol(self) -> char {
        match self {
            Self::Untracked => '?',
            Self::Added => 'A',
            Self::Renamed => 'R',
            Self::Deleted => 'D',
            Self::Modified => 'M',
            Self::Conflict => 'U',
        }
    }
}

/// Parsed result of one `git status`: a mark for every changed path **and**
/// every ancestor directory below the repository root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitStatus {
    marks: HashMap<PathBuf, GitMark>,
}

impl GitStatus {
    /// The marker for `path` (a changed file, or a directory containing one).
    #[must_use]
    pub fn mark(&self, path: &Path) -> Option<GitMark> {
        self.marks.get(path).copied()
    }

    /// `true` when nothing is modified.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.marks.is_empty()
    }

    /// Number of marked paths (files plus aggregated directories).
    #[must_use]
    pub fn len(&self) -> usize {
        self.marks.len()
    }

    /// Alias of [`is_clean`](Self::is_clean).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }

    fn insert(&mut self, root: &Path, rel: &str, mark: GitMark) {
        let mut path = root.join(rel.trim_end_matches('/'));
        // The file itself, then every ancestor up to (excluding) the root.
        self.marks
            .entry(path.clone())
            .and_modify(|m| *m = (*m).max(mark))
            .or_insert(mark);
        while path.pop() && path != root && path.starts_with(root) {
            self.marks
                .entry(path.clone())
                .and_modify(|m| *m = (*m).max(mark))
                .or_insert(mark);
        }
    }
}

/// Map a porcelain-v1 `XY` status pair to a mark.
fn classify(x: u8, y: u8) -> Option<GitMark> {
    match (x, y) {
        (b'?', b'?') => Some(GitMark::Untracked),
        (b'!', b'!') => None,
        (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D') => Some(GitMark::Conflict),
        (b'M', _) | (_, b'M') | (b'T', _) | (_, b'T') => Some(GitMark::Modified),
        (b'D', _) | (_, b'D') => Some(GitMark::Deleted),
        (b'R', _) | (_, b'R') | (b'C', _) | (_, b'C') => Some(GitMark::Renamed),
        (b'A', _) | (_, b'A') => Some(GitMark::Added),
        _ => None,
    }
}

/// Parse `git status --porcelain=v1 -z` output for a repository at `root`.
#[must_use]
pub fn parse_porcelain_z(root: &Path, output: &[u8]) -> GitStatus {
    let mut status = GitStatus::default();
    let mut records = output.split(|&b| b == 0).filter(|r| !r.is_empty());
    while let Some(rec) = records.next() {
        if rec.len() < 4 {
            continue;
        }
        let (x, y) = (rec[0], rec[1]);
        let path = String::from_utf8_lossy(&rec[3..]).into_owned();
        // Renames/copies are followed by a second NUL-separated "from" path.
        if x == b'R' || x == b'C' || y == b'R' || y == b'C' {
            let _ = records.next();
        }
        if let Some(mark) = classify(x, y) {
            status.insert(root, &path, mark);
        }
    }
    status
}

/// Run `git` in `dir` and return the repository status, or `None` when `dir`
/// is not inside a work tree or `git` is unavailable.
#[must_use]
pub fn status_for(dir: &Path) -> Option<GitStatus> {
    let top = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let root = PathBuf::from(String::from_utf8_lossy(&top.stdout).trim_end());
    // `rev-parse` may resolve symlinks; match the user's view of `dir`.
    let root = match dir.canonicalize() {
        Ok(canon_dir) if canon_dir.starts_with(&root) => {
            let rel_depth = canon_dir
                .strip_prefix(&root)
                .map_or(0, |p| p.components().count());
            let mut r = dir.to_path_buf();
            for _ in 0..rel_depth {
                r.pop();
            }
            r
        }
        _ => root,
    };
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=normal"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(parse_porcelain_z(&root, &out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from("/repo")
    }

    #[test]
    fn parses_basic_statuses() {
        let out = b" M src/lib.rs\0A  new.txt\0?? scratch.txt\0 D gone.txt\0";
        let s = parse_porcelain_z(&root(), out);
        assert_eq!(
            s.mark(Path::new("/repo/src/lib.rs")),
            Some(GitMark::Modified)
        );
        assert_eq!(s.mark(Path::new("/repo/new.txt")), Some(GitMark::Added));
        assert_eq!(
            s.mark(Path::new("/repo/scratch.txt")),
            Some(GitMark::Untracked)
        );
        assert_eq!(s.mark(Path::new("/repo/gone.txt")), Some(GitMark::Deleted));
    }

    #[test]
    fn directories_aggregate_highest_priority() {
        let out = b"?? a/b/new.txt\0 M a/c.rs\0";
        let s = parse_porcelain_z(&root(), out);
        assert_eq!(s.mark(Path::new("/repo/a")), Some(GitMark::Modified));
        assert_eq!(s.mark(Path::new("/repo/a/b")), Some(GitMark::Untracked));
        assert_eq!(
            s.mark(Path::new("/repo")),
            None,
            "root itself is not marked"
        );
    }

    #[test]
    fn rename_consumes_the_from_path() {
        let out = b"R  new_name.rs\0old_name.rs\0 M other.rs\0";
        let s = parse_porcelain_z(&root(), out);
        assert_eq!(
            s.mark(Path::new("/repo/new_name.rs")),
            Some(GitMark::Renamed)
        );
        assert_eq!(s.mark(Path::new("/repo/old_name.rs")), None);
        assert_eq!(s.mark(Path::new("/repo/other.rs")), Some(GitMark::Modified));
    }

    #[test]
    fn conflicts_win_over_modified() {
        let out = b"UU merge.rs\0";
        let s = parse_porcelain_z(&root(), out);
        assert_eq!(s.mark(Path::new("/repo/merge.rs")), Some(GitMark::Conflict));
        assert!(GitMark::Conflict > GitMark::Modified);
    }

    #[test]
    fn ignored_and_empty_input_yield_no_marks() {
        assert!(parse_porcelain_z(&root(), b"").is_clean());
        assert!(parse_porcelain_z(&root(), b"!! target/\0").is_clean());
    }

    #[test]
    fn status_for_a_missing_directory_is_none() {
        assert!(status_for(Path::new("/definitely/not/a/real/dir")).is_none());
    }

    #[test]
    fn status_for_real_repo_marks_untracked() {
        let tmp = tempfile::tempdir().unwrap();
        let ok = Command::new("git")
            .arg("-C")
            .arg(tmp.path())
            .args(["init", "-q"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return; // git unavailable
        }
        std::fs::write(tmp.path().join("hello.txt"), "x").unwrap();
        let status = status_for(tmp.path()).expect("status");
        assert_eq!(
            status.mark(&tmp.path().join("hello.txt")),
            Some(GitMark::Untracked)
        );
    }
}
