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
            let item_count = if is_dir {
                fs::read_dir(&path)
                    .ok()
                    .map(|items| items.flatten().count())
            } else {
                None
            };
            entries.push(FsEntry {
                name,
                path,
                is_dir,
                size: (!is_dir).then_some(metadata.len()),
                item_count,
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
        crate::fs::copy_dir_all(source, destination)
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
        crate::fs::disk_usage(path)
    }

    fn dir_size(&self, path: &Path) -> (u64, bool) {
        crate::fs::dir_size(path)
    }
}

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
