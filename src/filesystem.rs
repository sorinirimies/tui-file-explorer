use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{types::DiskUsage, FsEntry};

/// Filesystem backend used by explorer widgets.
///
/// Implement this trait to browse an in-memory, remote, archive, or otherwise
/// virtual filesystem while keeping the same state machine and renderer.
pub trait FileSystem: fmt::Debug + Send + Sync {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<FsEntry>>;
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn create_file(&self, path: &Path) -> io::Result<()>;
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()>;
    fn copy_file(&self, source: &Path, destination: &Path) -> io::Result<()>;
    fn copy_dir(&self, source: &Path, destination: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
    fn disk_usage(&self, path: &Path) -> Option<DiskUsage>;
    fn dir_size(&self, path: &Path) -> (u64, bool);

    /// Number of immediate children of the directory at `path`.
    ///
    /// Called lazily for the rows that are actually visible, so
    /// [`read_dir`](Self::read_dir) can leave
    /// [`FsEntry::item_count`] unset and stay fast on huge directories.
    /// The default reports "unknown".
    fn item_count(&self, _path: &Path) -> Option<usize> {
        None
    }

    /// Bring a previously trashed `path` back to where it was deleted from.
    ///
    /// Only trash-backed filesystems on platforms with a restorable trash
    /// (Linux, Windows) support this; the default reports
    /// [`io::ErrorKind::Unsupported`].
    fn restore_deleted(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "restoring deleted items is not supported here",
        ))
    }

    /// `true` when paths are real OS paths (symlink targets, `git status` and
    /// similar OS-level lookups are meaningful). Virtual backends keep the
    /// default `false`.
    fn is_local(&self) -> bool {
        false
    }
}

pub type SharedFileSystem = Arc<dyn FileSystem>;

/// Native local-filesystem backend.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<FsEntry>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)?.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let is_dir = metadata.is_dir();
            let extension = if is_dir {
                String::new()
            } else {
                path.extension()
                    .map(|value| value.to_string_lossy().to_lowercase())
                    .unwrap_or_default()
            };
            entries.push(FsEntry {
                name,
                path,
                is_dir,
                size: (!is_dir).then_some(metadata.len()),
                // Filled lazily for visible rows (see `FileSystem::item_count`).
                item_count: None,
                extension,
            });
        }
        Ok(entries)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn create_file(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map(|_| ())
    }

    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
        fs::rename(source, destination)
    }

    fn copy_file(&self, source: &Path, destination: &Path) -> io::Result<()> {
        fs::copy(source, destination).map(|_| ())
    }

    fn copy_dir(&self, source: &Path, destination: &Path) -> io::Result<()> {
        crate::fs_util::copy_dir_all(source, destination)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn disk_usage(&self, path: &Path) -> Option<DiskUsage> {
        crate::fs_util::disk_usage(path)
    }

    fn dir_size(&self, path: &Path) -> (u64, bool) {
        crate::fs_util::dir_size(path)
    }

    fn item_count(&self, path: &Path) -> Option<usize> {
        fs::read_dir(path).ok().map(|items| items.flatten().count())
    }

    fn is_local(&self) -> bool {
        true
    }
}

/// Local filesystem that sends removals to the OS trash / recycle bin.
///
/// Everything except [`remove_file`](FileSystem::remove_file) and
/// [`remove_dir_all`](FileSystem::remove_dir_all) behaves exactly like
/// [`StdFileSystem`]. If the trash is unavailable (e.g. a headless Linux
/// box without a trash directory) the removal **fails** rather than silently
/// falling back to permanent deletion.
#[cfg(feature = "trash")]
#[derive(Debug, Clone, Copy, Default)]
pub struct TrashFileSystem;

#[cfg(feature = "trash")]
impl FileSystem for TrashFileSystem {
    fn read_dir(&self, path: &Path) -> io::Result<Vec<FsEntry>> {
        StdFileSystem.read_dir(path)
    }
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        StdFileSystem.create_dir_all(path)
    }
    fn create_file(&self, path: &Path) -> io::Result<()> {
        StdFileSystem.create_file(path)
    }
    fn rename(&self, source: &Path, destination: &Path) -> io::Result<()> {
        StdFileSystem.rename(source, destination)
    }
    fn copy_file(&self, source: &Path, destination: &Path) -> io::Result<()> {
        StdFileSystem.copy_file(source, destination)
    }
    fn copy_dir(&self, source: &Path, destination: &Path) -> io::Result<()> {
        StdFileSystem.copy_dir(source, destination)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        trash::delete(path).map_err(io::Error::other)
    }
    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        trash::delete(path).map_err(io::Error::other)
    }
    fn exists(&self, path: &Path) -> bool {
        StdFileSystem.exists(path)
    }
    fn is_dir(&self, path: &Path) -> bool {
        StdFileSystem.is_dir(path)
    }
    fn disk_usage(&self, path: &Path) -> Option<DiskUsage> {
        StdFileSystem.disk_usage(path)
    }
    fn dir_size(&self, path: &Path) -> (u64, bool) {
        StdFileSystem.dir_size(path)
    }
    fn item_count(&self, path: &Path) -> Option<usize> {
        StdFileSystem.item_count(path)
    }
    fn is_local(&self) -> bool {
        true
    }

    #[cfg(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    ))]
    fn restore_deleted(&self, path: &Path) -> io::Result<()> {
        // Newest trashed item that came from `path`.
        let item = trash::os_limited::list()
            .map_err(io::Error::other)?
            .into_iter()
            .filter(|i| i.original_path() == path)
            .max_by_key(|i| i.time_deleted)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found in the trash"))?;
        trash::os_limited::restore_all([item]).map_err(io::Error::other)
    }
}

/// `true` when this build can send deletions to the OS trash.
pub const TRASH_SUPPORTED: bool = cfg!(feature = "trash");

/// Local filesystem backend, trash-backed when `use_trash` is requested *and*
/// the `trash` feature is compiled in; plain [`StdFileSystem`] otherwise.
#[must_use]
pub fn local_filesystem(use_trash: bool) -> SharedFileSystem {
    #[cfg(feature = "trash")]
    if use_trash {
        return Arc::new(TrashFileSystem);
    }
    let _ = use_trash;
    Arc::new(StdFileSystem)
}

#[must_use]
pub fn std_filesystem() -> SharedFileSystem {
    Arc::new(StdFileSystem)
}

/// Convenience type for virtual backends that need to construct entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub item_count: Option<usize>,
    pub extension: String,
}

impl From<VirtualEntry> for FsEntry {
    fn from(entry: VirtualEntry) -> Self {
        Self {
            name: entry.name,
            path: entry.path,
            is_dir: entry.is_dir,
            size: entry.size,
            item_count: entry.item_count,
            extension: entry.extension,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FileExplorer;

    #[derive(Debug)]
    struct MemoryFileSystem;

    impl FileSystem for MemoryFileSystem {
        fn read_dir(&self, path: &Path) -> io::Result<Vec<FsEntry>> {
            Ok(vec![VirtualEntry {
                name: "virtual.txt".into(),
                path: path.join("virtual.txt"),
                is_dir: false,
                size: Some(7),
                item_count: None,
                extension: "txt".into(),
            }
            .into()])
        }

        fn create_dir_all(&self, _: &Path) -> io::Result<()> {
            Ok(())
        }
        fn create_file(&self, _: &Path) -> io::Result<()> {
            Ok(())
        }
        fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
            Ok(())
        }
        fn copy_file(&self, _: &Path, _: &Path) -> io::Result<()> {
            Ok(())
        }
        fn copy_dir(&self, _: &Path, _: &Path) -> io::Result<()> {
            Ok(())
        }
        fn remove_file(&self, _: &Path) -> io::Result<()> {
            Ok(())
        }
        fn remove_dir_all(&self, _: &Path) -> io::Result<()> {
            Ok(())
        }
        fn exists(&self, _: &Path) -> bool {
            true
        }
        fn is_dir(&self, _: &Path) -> bool {
            false
        }
        fn disk_usage(&self, _: &Path) -> Option<DiskUsage> {
            None
        }
        fn dir_size(&self, _: &Path) -> (u64, bool) {
            (0, false)
        }
    }

    #[test]
    fn explorer_reads_from_custom_filesystem() {
        let explorer = FileExplorer::builder(PathBuf::from("virtual:/"))
            .filesystem(Arc::new(MemoryFileSystem))
            .try_build()
            .unwrap();
        assert_eq!(explorer.entries.len(), 1);
        assert_eq!(explorer.entries[0].name, "virtual.txt");
    }
}
