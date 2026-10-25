//! Archive support: list (for previews) and extract `.zip`, `.tar`, `.tar.gz`
//! / `.tgz`.
//!
//! Extraction is path-traversal safe: entries that would escape the
//! destination (`../evil`, absolute paths) are rejected by the underlying
//! readers (`zip`'s `enclosed_name`, `tar`'s `unpack_in`).

use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
};

/// Supported container formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
}

impl ArchiveFormat {
    /// Detect the format from a file name (case-insensitive).
    #[must_use]
    pub fn detect(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
        if name.ends_with(".zip") || name.ends_with(".jar") {
            Some(Self::Zip)
        } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            Some(Self::TarGz)
        } else if name.ends_with(".tar") {
            Some(Self::Tar)
        } else {
            None
        }
    }

    /// Human-readable name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Zip => "ZIP",
            Self::Tar => "TAR",
            Self::TarGz => "TAR.GZ",
        }
    }
}

/// One member of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

/// Contents of an archive (possibly truncated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveListing {
    pub format: ArchiveFormat,
    /// First [`MAX_LISTED`] entries.
    pub entries: Vec<ArchiveEntry>,
    /// Total number of members (may exceed `entries.len()`).
    pub total_entries: usize,
    /// Sum of uncompressed sizes of *all* members.
    pub total_size: u64,
}

/// Entries kept in a listing; the rest are only counted.
pub const MAX_LISTED: usize = 2000;

fn open_tar(path: &Path, gz: bool) -> io::Result<tar::Archive<Box<dyn Read>>> {
    let file = BufReader::new(File::open(path)?);
    let reader: Box<dyn Read> = if gz {
        Box::new(flate2::read::GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    Ok(tar::Archive::new(reader))
}

fn zip_err(e: zip::result::ZipError) -> io::Error {
    match e {
        zip::result::ZipError::Io(e) => e,
        other => io::Error::new(io::ErrorKind::InvalidData, other.to_string()),
    }
}

/// List the contents of the archive at `path`.
///
/// # Errors
///
/// Fails when the file is not a supported archive or cannot be read.
pub fn list(path: &Path) -> io::Result<ArchiveListing> {
    let format = ArchiveFormat::detect(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not a supported archive"))?;
    let mut listing = ArchiveListing {
        format,
        entries: Vec::new(),
        total_entries: 0,
        total_size: 0,
    };
    let mut push = |name: String, size: u64, is_dir: bool| {
        listing.total_entries = listing.total_entries.saturating_add(1);
        listing.total_size = listing.total_size.saturating_add(size);
        if listing.entries.len() < MAX_LISTED {
            listing.entries.push(ArchiveEntry { name, size, is_dir });
        }
    };

    match format {
        ArchiveFormat::Zip => {
            let mut zip =
                zip::ZipArchive::new(BufReader::new(File::open(path)?)).map_err(zip_err)?;
            for i in 0..zip.len() {
                let f = zip.by_index_raw(i).map_err(zip_err)?;
                push(f.name().map_err(zip_err)?.to_string(), f.size(), f.is_dir());
            }
        }
        ArchiveFormat::Tar | ArchiveFormat::TarGz => {
            let mut tar = open_tar(path, format == ArchiveFormat::TarGz)?;
            for entry in tar.entries()? {
                let entry = entry?;
                let name = entry.path()?.to_string_lossy().into_owned();
                push(name, entry.size(), entry.header().entry_type().is_dir());
            }
        }
    }
    Ok(listing)
}

/// The directory an archive extracts into by default: the archive's path
/// without its archive extension (`a/b/foo.tar.gz` → `a/b/foo`).
#[must_use]
pub fn default_destination(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    let stem_len = [".tar.gz", ".tgz", ".tar", ".zip", ".jar"]
        .iter()
        .find(|ext| lower.ends_with(**ext))
        .map_or(name.len(), |ext| name.len() - ext.len());
    let mut stem = name[..stem_len].to_string();
    if stem.is_empty() {
        stem = "extracted".into();
    }
    path.with_file_name(stem)
}

/// Maximum number of members [`extract`] will unpack (zip-bomb guard).
pub const MAX_EXTRACT_ENTRIES: usize = 100_000;
/// Maximum total uncompressed bytes [`extract`] will unpack (zip-bomb guard).
pub const MAX_EXTRACT_BYTES: u64 = 8 * 1024 * 1024 * 1024;

fn too_big(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("archive exceeds extraction limit ({what})"),
    )
}

/// Extract the archive at `path` into `dest` (created if missing).
///
/// Returns the number of members extracted. When extraction fails and `dest`
/// did not exist beforehand, the partially written `dest` is removed.
///
/// # Errors
///
/// Fails on unreadable/corrupt archives, when a member would escape `dest`,
/// or when the archive exceeds [`MAX_EXTRACT_ENTRIES`] / [`MAX_EXTRACT_BYTES`].
pub fn extract(path: &Path, dest: &Path) -> io::Result<usize> {
    let existed = dest.exists();
    let result = extract_inner(path, dest);
    if result.is_err() && !existed {
        let _ = std::fs::remove_dir_all(dest);
    }
    result
}

fn extract_inner(path: &Path, dest: &Path) -> io::Result<usize> {
    let format = ArchiveFormat::detect(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not a supported archive"))?;
    match format {
        ArchiveFormat::Zip => {
            let mut zip =
                zip::ZipArchive::new(BufReader::new(File::open(path)?)).map_err(zip_err)?;
            let n = zip.len();
            if n > MAX_EXTRACT_ENTRIES {
                return Err(too_big("entries"));
            }
            let mut total = 0u64;
            for i in 0..n {
                total = total.saturating_add(zip.by_index_raw(i).map_err(zip_err)?.size());
            }
            if total > MAX_EXTRACT_BYTES {
                return Err(too_big("bytes"));
            }
            std::fs::create_dir_all(dest)?;
            zip.extract(dest).map_err(zip_err)?;
            Ok(n)
        }
        ArchiveFormat::Tar | ArchiveFormat::TarGz => {
            std::fs::create_dir_all(dest)?;
            let mut tar = open_tar(path, format == ArchiveFormat::TarGz)?;
            let mut n = 0;
            let mut total = 0u64;
            for entry in tar.entries()? {
                let mut entry = entry?;
                total = total.saturating_add(entry.size());
                if n >= MAX_EXTRACT_ENTRIES {
                    return Err(too_big("entries"));
                }
                if total > MAX_EXTRACT_BYTES {
                    return Err(too_big("bytes"));
                }
                // `unpack_in` refuses paths that escape `dest`.
                if !entry.unpack_in(dest)? {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unsafe path in archive: {}", entry.path()?.display()),
                    ));
                }
                n += 1;
            }
            Ok(n)
        }
    }
}

#[cfg(test)]
mod tests;
