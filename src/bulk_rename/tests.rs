//! Unit tests for [`super`].

use super::*;
use crate::filesystem::StdFileSystem;
use std::fs;

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(|n| PathBuf::from("/d").join(n)).collect()
}

fn none(_: &Path) -> bool {
    false
}

#[test]
fn names_file_is_one_name_per_line() {
    assert_eq!(
        names_file_content(&paths(&["a.txt", "b"])).unwrap(),
        "a.txt\nb\n"
    );
}

#[test]
fn names_with_newlines_cannot_be_edited() {
    assert_eq!(names_file_content(&paths(&["a\nb"])).unwrap_err(), "a\nb");
}

#[test]
fn unchanged_text_plans_nothing() {
    let steps = plan(&paths(&["a", "b"]), "a\nb\n", none).unwrap();
    assert!(steps.is_empty());
}

#[test]
fn changed_lines_become_steps() {
    let steps = plan(&paths(&["a", "b"]), "a\nc\n", none).unwrap();
    assert_eq!(
        steps,
        vec![RenameStep {
            from: "/d/b".into(),
            to: "/d/c".into()
        }]
    );
}

#[test]
fn line_count_must_match() {
    assert_eq!(
        plan(&paths(&["a", "b"]), "a\n", none),
        Err(BulkRenameError::LineCount {
            expected: 2,
            found: 1
        })
    );
}

#[test]
fn blank_lines_are_rejected() {
    assert_eq!(
        plan(&paths(&["a", "b"]), "a\n\n", none),
        Err(BulkRenameError::EmptyName { line: 2 })
    );
}

#[test]
fn separators_and_dots_are_rejected() {
    for bad in ["x/y", "..", ".", "x\\y"] {
        assert!(
            matches!(
                plan(&paths(&["a"]), &format!("{bad}\n"), none),
                Err(BulkRenameError::InvalidName { line: 1, .. })
            ),
            "{bad}"
        );
    }
}

#[test]
fn duplicate_targets_are_rejected() {
    assert_eq!(
        plan(&paths(&["a", "b"]), "x\nx\n", none),
        Err(BulkRenameError::Duplicate { name: "x".into() })
    );
}

#[test]
fn existing_unrelated_target_is_rejected() {
    let taken = |p: &Path| p == Path::new("/d/taken");
    assert_eq!(
        plan(&paths(&["a"]), "taken\n", taken),
        Err(BulkRenameError::Exists {
            name: "taken".into()
        })
    );
}

#[test]
fn swapping_two_names_is_allowed() {
    let exists = |p: &Path| p == Path::new("/d/a") || p == Path::new("/d/b");
    let steps = plan(&paths(&["a", "b"]), "b\na\n", exists).unwrap();
    assert_eq!(steps.len(), 2);
}

#[test]
fn crlf_line_endings_are_tolerated() {
    let steps = plan(&paths(&["a"]), "b\r\n", none).unwrap();
    assert_eq!(steps[0].to, PathBuf::from("/d/b"));
}

fn temp_files(names: &[&str]) -> (tempfile::TempDir, Vec<PathBuf>) {
    let tmp = tempfile::tempdir().unwrap();
    let ps: Vec<PathBuf> = names
        .iter()
        .map(|n| {
            let p = tmp.path().join(n);
            fs::write(&p, n).unwrap();
            p
        })
        .collect();
    (tmp, ps)
}

#[test]
fn apply_renames_files() {
    let (tmp, ps) = temp_files(&["a.txt", "b.txt"]);
    let steps = plan(&ps, "x.txt\ny.txt\n", |p| p.exists()).unwrap();
    apply(&StdFileSystem, &steps).unwrap();
    assert_eq!(
        fs::read_to_string(tmp.path().join("x.txt")).unwrap(),
        "a.txt"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("y.txt")).unwrap(),
        "b.txt"
    );
    assert!(!tmp.path().join("a.txt").exists());
}

#[test]
fn apply_handles_swaps_without_losing_data() {
    let (tmp, ps) = temp_files(&["a", "b"]);
    let steps = plan(&ps, "b\na\n", |p| p.exists()).unwrap();
    apply(&StdFileSystem, &steps).unwrap();
    assert_eq!(fs::read_to_string(tmp.path().join("b")).unwrap(), "a");
    assert_eq!(fs::read_to_string(tmp.path().join("a")).unwrap(), "b");
}

#[test]
fn apply_handles_chains() {
    let (tmp, ps) = temp_files(&["a", "b"]);
    // a -> b, b -> c
    let steps = plan(&ps, "b\nc\n", |p| p.exists()).unwrap();
    apply(&StdFileSystem, &steps).unwrap();
    assert_eq!(fs::read_to_string(tmp.path().join("b")).unwrap(), "a");
    assert_eq!(fs::read_to_string(tmp.path().join("c")).unwrap(), "b");
    assert!(!tmp.path().join("a").exists());
}

#[test]
fn apply_rolls_back_when_a_source_vanished() {
    let (tmp, ps) = temp_files(&["a", "b"]);
    let steps = plan(&ps, "x\ny\n", |p| p.exists()).unwrap();
    fs::remove_file(&ps[1]).unwrap(); // second source disappears
    assert!(apply(&StdFileSystem, &steps).is_err());
    // The first entry must be back under its original name.
    assert_eq!(fs::read_to_string(tmp.path().join("a")).unwrap(), "a");
    assert!(!tmp.path().join("x").exists());
}
