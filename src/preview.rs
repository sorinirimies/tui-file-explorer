//! File preview pane for the TUI file explorer.
//!
//! Supports four content kinds:
//!
//! * **Text** — UTF-8 files with line numbers and scrolling.
//! * **Image** — raster images rendered via Unicode half-block (`▀`) pixels.
//! * **Binary** — classic hex dump (16 bytes per line, ASCII sidebar).
//! * **Directory** — entry / file / dir counts and total size.
//!
//! The public entry-points are [`PreviewState`] (owns cached content and scroll
//! position) and [`render_preview`] (draws into a [`ratatui::Frame`]).

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::palette::Theme;

// ── Constants ─────────────────────────────────────────────────────────────────

/// File extensions recognised as raster images.
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "tif", "ico", "svg",
];

/// Maximum number of bytes read for a text preview.
const MAX_PREVIEW_BYTES: usize = 512 * 1024; // 512 KB

/// Maximum number of lines kept for a text preview.
const MAX_PREVIEW_LINES: usize = 10_000;

/// Number of leading bytes inspected when deciding "text vs binary".
const BINARY_CHECK_BYTES: usize = 8192;

/// Number of raw bytes shown in the binary hex-dump view.
const BINARY_DUMP_BYTES: usize = 4096;

// ── File-type helpers ─────────────────────────────────────────────────────────

/// Returns `true` when `ext` (case-insensitive) matches a known image format.
#[must_use]
pub fn is_image_extension(ext: &str) -> bool {
    let lower = ext.to_ascii_lowercase();
    IMAGE_EXTENSIONS.iter().any(|e| *e == lower)
}

/// Heuristic: a buffer is "likely binary" when it contains at least one null
/// byte within the first [`BINARY_CHECK_BYTES`].
fn is_likely_binary(data: &[u8]) -> bool {
    let limit = data.len().min(BINARY_CHECK_BYTES);
    data[..limit].contains(&0)
}

// ── Preview content types ─────────────────────────────────────────────────────

/// The payload of a preview pane.
pub enum PreviewContent {
    /// UTF-8 text with line data.
    Text(TextPreview),
    /// Decoded raster image (RGB pixels, already resized to fit the area).
    Image(Box<ImagePreview>),
    /// Hex dump of the first few kilobytes.
    Binary(BinaryPreview),
    /// Directory statistics.
    Directory(DirPreview),
    /// Listing of a `.zip` / `.tar` / `.tar.gz` archive.
    Archive(crate::archive::ArchiveListing),
    /// The content is being loaded on a background thread.
    Loading,
    /// Nothing selected.
    Empty,
    /// File too large for a text preview.
    TooLarge {
        /// Size in bytes.
        size: u64,
    },
    /// An I/O or decode error.
    Error(String),
}

/// Text file preview data.
pub struct TextPreview {
    /// Individual lines (no trailing newline).
    pub lines: Vec<String>,
    /// Total number of lines in the file (may exceed `lines.len()` when
    /// truncated by `MAX_PREVIEW_LINES`).
    pub total_lines: usize,
}

/// Image rendered via ratatui-image (supports Kitty/Sixel/iTerm2/halfblocks).
pub struct ImagePreview {
    /// Stateful protocol handle used by `ratatui-image` to render the image.
    /// Wrapped in a `RefCell` so we can obtain `&mut` during rendering even
    /// when the preview state is shared.
    pub protocol: std::cell::RefCell<ratatui_image::protocol::StatefulProtocol>,
}

/// Binary hex-dump preview data.
pub struct BinaryPreview {
    /// Pre-formatted hex lines (offset + hex + ASCII).
    pub hex_lines: Vec<String>,
    /// Total size of the file in bytes.
    pub total_bytes: u64,
}

/// Directory statistics.
pub struct DirPreview {
    /// Number of direct children.
    pub entry_count: usize,
    /// Number of child files.
    pub file_count: usize,
    /// Number of child directories.
    pub dir_count: usize,
    /// Cumulative size of direct child files (non-recursive).
    pub total_size: u64,
}

// ── Loaded (thread-safe intermediate) ─────────────────────────────────────────

/// Result of a background load. Unlike [`PreviewContent`] it is `Send`: an
/// image is carried as decoded pixels and turned into a terminal protocol
/// object on the UI thread (which owns the `Picker`).
enum Loaded {
    Text(TextPreview),
    Image(image::DynamicImage),
    Binary(BinaryPreview),
    Directory(DirPreview),
    Archive(crate::archive::ArchiveListing),
    TooLarge { size: u64 },
    Error(String),
}

impl Loaded {
    fn into_content(self, picker: &mut ratatui_image::picker::Picker) -> PreviewContent {
        match self {
            Self::Text(t) => PreviewContent::Text(t),
            Self::Image(img) => PreviewContent::Image(Box::new(ImagePreview {
                protocol: std::cell::RefCell::new(picker.new_resize_protocol(img)),
            })),
            Self::Binary(b) => PreviewContent::Binary(b),
            Self::Directory(d) => PreviewContent::Directory(d),
            Self::Archive(a) => PreviewContent::Archive(a),
            Self::TooLarge { size } => PreviewContent::TooLarge { size },
            Self::Error(e) => PreviewContent::Error(e),
        }
    }
}

// ── PreviewState ──────────────────────────────────────────────────────────────

/// Owns the cached preview content, the path it was generated for, and the
/// current scroll offset.
pub struct PreviewState {
    /// Path for which the current content was loaded, or `None`.
    pub cached_path: Option<PathBuf>,
    /// The loaded content.
    pub content: PreviewContent,
    /// Vertical scroll offset (in lines / hex rows / pixel-rows, depending on
    /// content kind).
    pub scroll: usize,
    /// Width of the area the content was rendered into last time.
    cached_width: u16,
    /// Height of the area the content was rendered into last time.
    cached_height: u16,
    /// Image protocol picker (detects best protocol for the terminal).
    picker: ratatui_image::picker::Picker,
    /// In-flight background load, if any.
    task: Option<crate::worker::Task<Loaded>>,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewState {
    /// Create a blank preview state.
    ///
    /// Falls back to halfblocks because `from_query_stdio` must be called
    /// **before** the terminal enters the alternate screen.  Use
    /// [`PreviewState::with_picker`] when you can create the picker early.
    #[must_use]
    pub fn new() -> Self {
        Self::with_picker(ratatui_image::picker::Picker::halfblocks())
    }

    /// Create a preview state with a pre-initialised image-protocol picker.
    ///
    /// Call [`ratatui_image::picker::Picker::from_query_stdio`] (or similar)
    /// **before** entering the alternate screen and pass the result here.
    #[must_use]
    pub fn with_picker(picker: ratatui_image::picker::Picker) -> Self {
        Self {
            cached_path: None,
            content: PreviewContent::Empty,
            scroll: 0,
            cached_width: 0,
            cached_height: 0,
            picker,
            task: None,
        }
    }

    /// Return the detected image protocol type (for diagnostics / UI display).
    #[must_use]
    pub fn protocol_type(&self) -> ratatui_image::picker::ProtocolType {
        self.picker.protocol_type()
    }

    /// Reload the preview if the path changed or the available area was
    /// resized.  When neither changed, this is a no-op.
    ///
    /// Loading happens on a background thread: after a path change the
    /// content is [`PreviewContent::Loading`] until [`poll`](Self::poll)
    /// (called by this method on every frame) picks the result up.
    pub fn update(&mut self, path: Option<&Path>, area_width: u16, area_height: u16) {
        self.poll();

        let path_changed = match (&self.cached_path, path) {
            (Some(cached), Some(new)) => cached != new,
            (None, None) => false,
            _ => true,
        };

        let size_changed = self.cached_width != area_width || self.cached_height != area_height;

        if !path_changed && !size_changed {
            return;
        }

        self.scroll = 0;
        self.cached_width = area_width;
        self.cached_height = area_height;

        if let Some(p) = path {
            self.cached_path = Some(p.to_path_buf());
            // Keep showing the old content on a pure resize.
            if path_changed {
                self.content = PreviewContent::Loading;
            }
            let target = p.to_path_buf();
            // Replacing the task drops (cancels) any previous one.
            self.task = Some(crate::worker::Task::spawn(move |emit| {
                emit.send(load_preview(&target));
            }));
        } else {
            self.task = None;
            self.cached_path = None;
            self.content = PreviewContent::Empty;
        }
    }

    /// Collect a finished background load. Returns `true` when the content
    /// changed.
    pub fn poll(&mut self) -> bool {
        let Some(task) = &mut self.task else {
            return false;
        };
        let mut changed = false;
        for loaded in task.poll() {
            self.content = loaded.into_content(&mut self.picker);
            changed = true;
        }
        if task.is_done() {
            self.task = None;
        }
        changed
    }

    /// `true` while a background load is running.
    #[must_use]
    pub fn is_loading(&self) -> bool {
        self.task.is_some()
    }

    /// Block until the current load finishes (or `timeout` passes).
    ///
    /// For tests and headless callers; interactive UIs rely on
    /// [`poll`](Self::poll) per frame.
    pub fn wait_until_loaded(&mut self, timeout: std::time::Duration) {
        let deadline = std::time::Instant::now() + timeout;
        while self.is_loading() && std::time::Instant::now() < deadline {
            self.poll();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        self.poll();
    }

    /// Scroll the preview up by `n` rows (clamped at zero).
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    /// Scroll the preview down by `n` rows.
    pub fn scroll_down(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_add(n);
    }

    /// Mark the cache as stale so that the next [`update`](Self::update) call
    /// reloads even if the path has not changed.
    pub fn invalidate(&mut self) {
        self.task = None;
        self.cached_path = None;
        self.cached_width = 0;
        self.cached_height = 0;
    }
}

// ── Loading helpers (private) ─────────────────────────────────────────────────

/// Top-level dispatcher: decide which loader to call based on metadata and
/// extension.
fn load_preview(path: &Path) -> Loaded {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return Loaded::Error(format!("{e}")),
    };

    if meta.is_dir() {
        return load_dir_preview(path);
    }

    // Check for image extension.
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if is_image_extension(ext) {
            return load_image_preview(path);
        }
    }

    // Archives are listed, not dumped as text/hex.
    if crate::archive::ArchiveFormat::detect(path).is_some() {
        return match crate::archive::list(path) {
            Ok(listing) => Loaded::Archive(listing),
            Err(e) => Loaded::Error(format!("archive: {e}")),
        };
    }

    // Guard against very large files.
    if meta.len() > MAX_PREVIEW_BYTES as u64 {
        return Loaded::TooLarge { size: meta.len() };
    }

    load_text_preview(path)
}

/// Read a UTF-8 text file, capping at [`MAX_PREVIEW_LINES`].  Falls back to
/// binary preview if null bytes are detected in the first chunk.
fn load_text_preview(path: &Path) -> Loaded {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Loaded::Error(format!("{e}")),
    };
    let mut reader = BufReader::new(file);

    // Sniff the first bytes for binary content.
    let buf = reader.fill_buf();
    match buf {
        Ok(data) => {
            if is_likely_binary(data) {
                return load_binary_preview(path);
            }
        }
        Err(e) => return Loaded::Error(format!("{e}")),
    }

    // Re-open from the start (fill_buf does not consume, but to be safe with
    // large files we re-open).
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Loaded::Error(format!("{e}")),
    };
    let reader = BufReader::new(file);

    let mut lines = Vec::new();
    let mut total_lines = 0usize;
    for maybe_line in reader.lines() {
        match maybe_line {
            Ok(line) => {
                total_lines += 1;
                if lines.len() < MAX_PREVIEW_LINES {
                    lines.push(line);
                }
            }
            Err(_) => {
                // Non-UTF-8 — fall back to binary.
                return load_binary_preview(path);
            }
        }
    }

    Loaded::Text(TextPreview { lines, total_lines })
}

/// Decode a raster image (the terminal protocol is built on the UI thread).
fn load_image_preview(path: &Path) -> Loaded {
    match image::open(path) {
        Ok(img) => Loaded::Image(img),
        Err(e) => Loaded::Error(format!("image decode: {e}")),
    }
}

/// Read the first [`BINARY_DUMP_BYTES`] and format a classic hex dump.
fn load_binary_preview(path: &Path) -> Loaded {
    let total_bytes = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => return Loaded::Error(format!("{e}")),
    };

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Loaded::Error(format!("{e}")),
    };

    let mut buf = vec![0u8; BINARY_DUMP_BYTES];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(e) => return Loaded::Error(format!("{e}")),
    };
    buf.truncate(n);

    let hex_lines = format_hex_dump(&buf);

    Loaded::Binary(BinaryPreview {
        hex_lines,
        total_bytes,
    })
}

/// Produce hex-dump lines: `OFFSET  HH HH … HH  |ASCII...|`
fn format_hex_dump(data: &[u8]) -> Vec<String> {
    let mut lines = Vec::with_capacity(data.len().div_ceil(16));
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = i * 16;
        let mut hex_part = String::with_capacity(48);
        let mut ascii_part = String::with_capacity(16);

        for (j, &byte) in chunk.iter().enumerate() {
            if j == 8 {
                hex_part.push(' ');
            }
            hex_part.push_str(&format!("{byte:02x} "));

            ascii_part.push(if byte.is_ascii_graphic() || byte == b' ' {
                byte as char
            } else {
                '.'
            });
        }

        // Pad the hex section to a fixed width (49 chars: 16*3 + 1 mid-space).
        while hex_part.len() < 49 {
            hex_part.push(' ');
        }

        lines.push(format!("{offset:08x}  {hex_part} |{ascii_part}|"));
    }
    lines
}

/// Walk the immediate children of a directory and gather counts + sizes.
fn load_dir_preview(path: &Path) -> Loaded {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => return Loaded::Error(format!("{e}")),
    };

    let mut entry_count = 0usize;
    let mut file_count = 0usize;
    let mut dir_count = 0usize;
    let mut total_size = 0u64;

    for entry in entries.flatten() {
        entry_count += 1;
        if let Ok(meta) = entry.metadata() {
            if meta.is_dir() {
                dir_count += 1;
            } else {
                file_count += 1;
                total_size += meta.len();
            }
        }
    }

    Loaded::Directory(DirPreview {
        entry_count,
        file_count,
        dir_count,
        total_size,
    })
}

// ── Public rendering entry-point ──────────────────────────────────────────────

/// Render the preview pane into `area`.
///
/// Draws a single-line header (file name) and the content below.  The
/// appearance is driven by `theme`.
pub fn render_preview(frame: &mut Frame, area: Rect, state: &PreviewState, theme: &Theme) {
    if area.width < 3 || area.height < 3 {
        return;
    }

    // ── Split: header (1 line) + content ──────────────────────────────────
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    let header_area = chunks[0];
    let content_area = chunks[1];

    // ── Header ────────────────────────────────────────────────────────────
    let title = match &state.cached_path {
        Some(p) => p.file_name().map_or_else(
            || p.display().to_string(),
            |n| n.to_string_lossy().to_string(),
        ),
        None => String::from("No selection"),
    };

    // Show detected image protocol in the header when previewing an image.
    let proto_hint = if matches!(&state.content, PreviewContent::Image(_)) {
        let proto = state.protocol_type();
        format!("  [{proto:?}]")
    } else {
        String::new()
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {title}"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(proto_hint, Style::default().fg(theme.dim)),
    ]))
    .alignment(Alignment::Left);
    frame.render_widget(header, header_area);

    // ── Content ───────────────────────────────────────────────────────────
    match &state.content {
        PreviewContent::Text(tp) => render_text(frame, content_area, tp, state.scroll, theme),
        PreviewContent::Image(ip) => render_image(frame, content_area, ip, theme),
        PreviewContent::Binary(bp) => render_binary(frame, content_area, bp, state.scroll, theme),
        PreviewContent::Directory(dp) => render_directory(frame, content_area, dp, theme),
        PreviewContent::Archive(al) => render_archive(frame, content_area, al, state.scroll, theme),
        PreviewContent::Loading => render_info(frame, content_area, "Loading…", theme),
        PreviewContent::Empty => render_info(frame, content_area, "Nothing selected", theme),
        PreviewContent::TooLarge { size } => {
            let msg = format!("File too large for preview ({size} bytes)");
            render_info(frame, content_area, &msg, theme);
        }
        PreviewContent::Error(msg) => render_info(frame, content_area, msg, theme),
    }
}

// ── Content renderers (private) ───────────────────────────────────────────────

/// Line-numbered text with scroll offset.
fn render_text(frame: &mut Frame, area: Rect, preview: &TextPreview, scroll: usize, theme: &Theme) {
    let gutter_width = 5u16; // "NNNN " — 4 digits + space
    if area.width <= gutter_width {
        return;
    }

    let visible = area.height as usize;
    let start = scroll.min(preview.lines.len().saturating_sub(1));
    let end = (start + visible).min(preview.lines.len());

    let lines: Vec<Line<'_>> = preview.lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let line_no = start + i + 1;
            Line::from(vec![
                Span::styled(format!("{line_no:>4} "), Style::default().fg(theme.dim)),
                Span::styled(text.as_str(), Style::default().fg(theme.fg)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);

    // Scroll thumb.
    crate::render::paint_scrollbar(frame, area, preview.total_lines, scroll, theme.accent);
}

/// Archive member list with a summary line, scrollable.
fn render_archive(
    frame: &mut Frame,
    area: Rect,
    listing: &crate::archive::ArchiveListing,
    scroll: usize,
    theme: &Theme,
) {
    if area.height < 2 {
        return;
    }
    let summary = format!(
        " {} · {} item(s) · {} uncompressed{}",
        listing.format.label(),
        listing.total_entries,
        crate::explorer::fmt_size(listing.total_size),
        if listing.total_entries > listing.entries.len() {
            format!(" · showing first {}", listing.entries.len())
        } else {
            String::new()
        },
    );
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);
    frame.render_widget(
        Paragraph::new(Span::styled(summary, Style::default().fg(theme.dim))),
        chunks[0],
    );

    let visible = usize::from(chunks[1].height);
    let start = scroll.min(listing.entries.len().saturating_sub(1));
    let lines: Vec<Line<'_>> = listing
        .entries
        .iter()
        .skip(start)
        .take(visible)
        .map(|e| {
            let name_style = if e.is_dir {
                Style::default().fg(theme.dir).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            let size = if e.is_dir {
                String::new()
            } else {
                format!("  {}", crate::explorer::fmt_size(e.size))
            };
            Line::from(vec![
                Span::styled(e.name.as_str(), name_style),
                Span::styled(size, Style::default().fg(theme.dim)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), chunks[1]);
    crate::render::paint_scrollbar(
        frame,
        chunks[1],
        listing.entries.len(),
        scroll,
        theme.accent,
    );
}

/// Image inside a bordered block, rendered via ratatui-image.
fn render_image(frame: &mut Frame, area: Rect, preview: &ImagePreview, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.dim))
        .title(Span::styled(
            " Image Preview ",
            Style::default().fg(theme.dim),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let image_widget = ratatui_image::StatefulImage::default();
    frame.render_stateful_widget(image_widget, inner, &mut *preview.protocol.borrow_mut());
}

/// Hex-dump paragraph with scroll.
fn render_binary(
    frame: &mut Frame,
    area: Rect,
    preview: &BinaryPreview,
    scroll: usize,
    theme: &Theme,
) {
    let visible = area.height as usize;
    let start = scroll.min(preview.hex_lines.len().saturating_sub(1));
    let end = (start + visible).min(preview.hex_lines.len());

    let lines: Vec<Line<'_>> = preview.hex_lines[start..end]
        .iter()
        .map(|l| Line::from(Span::styled(l.as_str(), Style::default().fg(theme.fg))))
        .collect();

    let footer = Line::from(Span::styled(
        format!(" total: {} bytes", preview.total_bytes),
        Style::default().fg(theme.dim),
    ));

    let mut all = lines;
    all.push(Line::from(""));
    all.push(footer);

    let paragraph = Paragraph::new(all);
    frame.render_widget(paragraph, area);

    // Scroll thumb.
    crate::render::paint_scrollbar(frame, area, preview.hex_lines.len(), scroll, theme.accent);
}

/// Directory statistics.
fn render_directory(frame: &mut Frame, area: Rect, preview: &DirPreview, theme: &Theme) {
    let info = vec![
        Line::from(vec![
            Span::styled("  Entries: ", Style::default().fg(theme.dim)),
            Span::styled(
                preview.entry_count.to_string(),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("    Files: ", Style::default().fg(theme.dim)),
            Span::styled(
                preview.file_count.to_string(),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("     Dirs: ", Style::default().fg(theme.dim)),
            Span::styled(preview.dir_count.to_string(), Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("    Size:  ", Style::default().fg(theme.dim)),
            Span::styled(
                format_size(preview.total_size),
                Style::default().fg(theme.fg),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(info);
    frame.render_widget(paragraph, area);
}

/// Generic info / error message centred in the area.
fn render_info(frame: &mut Frame, area: Rect, message: &str, theme: &Theme) {
    let paragraph = Paragraph::new(Line::from(Span::styled(
        message,
        Style::default().fg(theme.dim),
    )))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Human-readable byte size (simple, no external crate).
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
