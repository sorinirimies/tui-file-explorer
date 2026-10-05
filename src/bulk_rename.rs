//! Bulk rename: edit many names at once in a text editor.
//!
//! The flow (à la `vidir`): write the current names, one per line, to a temp
//! file; let the user edit it; read it back and rename every entry whose line
//! changed. [`plan`] validates the edit and [`apply`] performs it safely, in
//! two phases so swaps and chains (`a→b`, `b→a`) cannot clobber each other.

use std::{
    collections::HashSet,
    fmt, io,
    path::{Path, PathBuf},
};

use crate::filesystem::FileSystem;

/// Why an edited name list was rejected. Nothing is renamed in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkRenameError {
    /// The number of lines changed (lines must not be added or removed).
    LineCount { expected: usize, found: usize },
    /// A line was blanked.
    EmptyName { line: usize },
    /// A name contains a path separator (moving between directories is not
    /// supported) or is `.` / `..`.
    InvalidName { line: usize, name: String },
    /// Two entries would end up with the same name.
    Duplicate { name: String },
    /// A new name is already taken by an entry that is not being renamed.
    Exists { name: String },
}

impl fmt::Display for BulkRenameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineCount { expected, found } => {
                write!(
                    f,
                    "expected {expected} lines, found {found} — don't add or remove lines"
                )
            }
            Self::EmptyName { line } => write!(f, "line {line} is empty"),
            Self::InvalidName { line, name } => {
                write!(f, "line {line}: '{name}' is not a plain file name")
            }
            Self::Duplicate { name } => write!(f, "'{name}' would be used twice"),
            Self::Exists { name } => write!(f, "'{name}' already exists"),
        }
    }
}

impl std::error::Error for BulkRenameError {}

/// One validated rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameStep {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// The text to put in the editor: one file name per line.
///
/// # Errors
///
/// Returns the offending name when it contains a newline (it could not be
/// represented one-per-line).
pub fn names_file_content(sources: &[PathBuf]) -> Result<String, String> {
    let mut out = String::new();
    for p in sources {
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        if name.contains('\n') || name.contains('\r') {
            return Err(name.into_owned());
        }
        out.push_str(&name);
        out.push('\n');
    }
    Ok(out)
}

/// Validate `edited` (the editor's text) against `sources` and compute the
/// renames to perform. Unchanged lines produce no step.
///
/// `exists` reports whether a path is already occupied.
///
/// # Errors
///
/// See [`BulkRenameError`].
pub fn plan(
    sources: &[PathBuf],
    edited: &str,
    exists: impl Fn(&Path) -> bool,
) -> Result<Vec<RenameStep>, BulkRenameError> {
    let lines: Vec<&str> = edited.lines().collect();
    if lines.len() != sources.len() {
        return Err(BulkRenameError::LineCount {
            expected: sources.len(),
            found: lines.len(),
        });
    }

    let mut seen: HashSet<&str> = HashSet::new();
    for (i, raw) in lines.iter().enumerate() {
        let name = raw.trim_end_matches('\r').trim();
        if name.is_empty() {
            return Err(BulkRenameError::EmptyName { line: i + 1 });
        }
        if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
            return Err(BulkRenameError::InvalidName {
                line: i + 1,
                name: name.to_string(),
            });
        }
        if !seen.insert(name) {
            return Err(BulkRenameError::Duplicate {
                name: name.to_string(),
            });
        }
    }

    let moving: HashSet<&Path> = sources.iter().map(PathBuf::as_path).collect();
    let mut steps = Vec::new();
    for (src, raw) in sources.iter().zip(&lines) {
        let name = raw.trim_end_matches('\r').trim();
        let old = src.file_name().unwrap_or_default().to_string_lossy();
        if old == name {
            continue;
        }
        let to = src.with_file_name(name);
        // Taken by something that is not itself being renamed away.
        if exists(&to) && !moving.contains(to.as_path()) {
            return Err(BulkRenameError::Exists {
                name: name.to_string(),
            });
        }
        steps.push(RenameStep {
            from: src.clone(),
            to,
        });
    }
    Ok(steps)
}

/// Perform `steps` in two phases (everything to a unique temporary name,
/// then to the final name). On failure the already-done renames are rolled
/// back as far as possible.
///
/// Returns the steps that were applied.
///
/// # Errors
///
/// The first I/O error; state is restored best-effort.
pub fn apply(fs: &dyn FileSystem, steps: &[RenameStep]) -> io::Result<Vec<RenameStep>> {
    let tag = std::process::id();
    let temp_for = |i: usize, s: &RenameStep| s.from.with_file_name(format!(".tfe-bulk-{tag}-{i}"));

    // Phase 1: move everything out of the way.
    let mut staged: Vec<(usize, PathBuf)> = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        let tmp = temp_for(i, s);
        if let Err(e) = fs.rename(&s.from, &tmp) {
            rollback(fs, steps, &staged);
            return Err(e);
        }
        staged.push((i, tmp));
    }
    // Phase 2: temporary → final.
    let mut finished: Vec<usize> = Vec::new();
    for (i, tmp) in &staged {
        let s = &steps[*i];
        if let Err(e) = fs.rename(tmp, &s.to) {
            // Undo finals, then the staged ones.
            for &j in finished.iter().rev() {
                let _ = fs.rename(&steps[j].to, &staged[j].1);
            }
            rollback(fs, steps, &staged);
            return Err(e);
        }
        finished.push(*i);
    }
    Ok(steps.to_vec())
}

fn rollback(fs: &dyn FileSystem, steps: &[RenameStep], staged: &[(usize, PathBuf)]) {
    for (i, tmp) in staged.iter().rev() {
        let _ = fs.rename(tmp, &steps[*i].from);
    }
}

#[cfg(test)]
mod tests;
