use super::*;

/// Read `dir`, apply all active filters, sort entries, and return the result.
///
/// * Hidden entries are excluded unless `show_hidden` is `true`.
/// * When `ext_filter` is non-empty only files whose extension is in the list
///   are included (directories are always included).
/// * When `search_query` is non-empty only entries whose name contains the
///   query (case-insensitive) are included.
/// * Entries are sorted according to `sort_mode`; directories are always
///   placed before files regardless of the sort mode.
pub(crate) fn load_entries(
    dir: &Path,
    show_hidden: bool,
    ext_filter: &[String],
    sort_mode: SortMode,
    search_query: &str,
) -> Vec<FsEntry> {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut dirs: Vec<FsEntry> = Vec::new();
    let mut files: Vec<FsEntry> = Vec::new();

    for entry in read.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();
        let extension = if is_dir {
            String::new()
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        };

        // Extension filter — applied to files only; directories always pass.
        if !is_dir && !ext_filter.is_empty() {
            let matches = ext_filter
                .iter()
                .any(|f| f.eq_ignore_ascii_case(&extension));
            if !matches {
                continue;
            }
        }

        // Search query filter — applied to both files and directories.
        if !search_query.is_empty() {
            let q = search_query.to_lowercase();
            if !name.to_lowercase().contains(&q) {
                continue;
            }
        }

        let size = if is_dir {
            None
        } else {
            entry.metadata().ok().map(|m| m.len())
        };

        // Shallow (non-recursive) item count for directories — cheap enough
        // to compute on every listing since it only reads one directory
        // level, unlike a full recursive byte-size walk.
        let item_count = if is_dir {
            fs::read_dir(&path).ok().map(|rd| rd.flatten().count())
        } else {
            None
        };

        let fs_entry = FsEntry {
            name,
            path,
            is_dir,
            size,
            item_count,
            extension,
        };

        if is_dir {
            dirs.push(fs_entry);
        } else {
            files.push(fs_entry);
        }
    }

    // Sort each group according to the active mode.
    // Directories always sort alphabetically among themselves.
    dirs.sort_by_key(|a| a.name.to_lowercase());

    match sort_mode {
        SortMode::Name => {
            files.sort_by_key(|a| a.name.to_lowercase());
        }
        SortMode::SizeDesc => {
            // Largest first; treat missing size as 0.
            files.sort_by_key(|b| std::cmp::Reverse(b.size.unwrap_or(0)));
        }
        SortMode::Extension => {
            // By extension first, then by name within each extension group.
            files.sort_by(|a, b| {
                a.extension
                    .cmp(&b.extension)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
        }
    }

    // Dirs first, then sorted files.
    dirs.extend(files);
    dirs
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Declarative icon map — maps file extensions to emoji icons.
macro_rules! icon_map {
    ( dir => $dir_icon:expr, $( [ $( $ext:literal ),+ ] => $icon:expr, )* default => $fallback:expr $(,)? ) => {
        pub fn entry_icon(entry: &FsEntry) -> &'static str {
            if entry.is_dir {
                return $dir_icon;
            }
            match entry.extension.as_str() {
                $( $( $ext )|+ => $icon, )*
                _ => $fallback,
            }
        }
    };
}

icon_map! {
    dir => "📁",

    // Disk images
    ["iso", "dmg"]          => "💿",
    ["img"]                 => "🖼 ",
    // Archives
    ["zip", "gz", "xz", "zst", "bz2", "tar", "7z", "rar", "tgz", "tbz2"] => "📦",
    // Documents
    ["pdf"]                 => "📕",
    ["txt", "log", "rst"]   => "📄",
    ["md", "mdx", "markdown"] => "📝",
    // Config / data
    ["toml", "yaml", "yml", "json", "xml", "ini", "cfg", "conf", "env"] => "⚙ ",
    ["lock"]                => "🔒",
    // Source — languages
    ["rs"]                  => "🦀",
    ["py", "pyw"]           => "🐍",
    ["js", "mjs", "cjs", "ts", "mts", "cts", "jsx", "tsx", "go",
     "c", "h", "cpp", "cc", "cxx", "hpp", "hxx",
     "java", "kt", "kts", "rb", "erb", "php", "swift", "cs",
     "lua", "zig", "ex", "exs", "hs", "lhs", "ml", "mli"] => "📜",
    // Shell scripts
    ["sh", "bash", "zsh", "fish", "nu", "bat", "cmd", "ps1"] => "📜",
    // Web
    ["html", "htm", "xhtml"] => "🌐",
    ["css", "scss", "sass", "less", "svg"] => "🎨",
    // Images (raster)
    ["png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "tiff", "tif", "avif", "heic", "heif"] => "🖼 ",
    // Video
    ["mp4", "mkv", "avi", "mov", "webm", "flv", "wmv", "m4v"] => "🎬",
    // Audio
    ["mp3", "wav", "flac", "ogg", "aac", "m4a", "opus", "wma"] => "🎵",
    // Fonts
    ["ttf", "otf", "woff", "woff2", "eot"] => "🔤",
    // Executables / binaries
    ["exe", "msi", "deb", "rpm", "appimage", "apk"] => "⚙ ",

    default => "📄",
}

/// Format a byte count as a human-readable size string.
///
/// Exposed as a public helper so that custom renderers can reuse the same
/// formatting logic without reimplementing it.
///
/// ```
/// use tui_file_explorer::fmt_size;
///
/// assert_eq!(fmt_size(512),           "512 B");
/// assert_eq!(fmt_size(1_536),         "1.5 KB");
/// assert_eq!(fmt_size(2_097_152),     "2.0 MB");
/// assert_eq!(fmt_size(1_073_741_824), "1.0 GB");
/// ```
pub fn fmt_size(bytes: u64) -> String {
    const KB: u64 = 1_024;
    const MB: u64 = 1_024 * KB;
    const GB: u64 = 1_024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
