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
    /// truncated by [`MAX_PREVIEW_LINES`]).
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
    pub fn new() -> Self {
        Self::with_picker(ratatui_image::picker::Picker::halfblocks())
    }

    /// Create a preview state with a pre-initialised image-protocol picker.
    ///
    /// Call [`ratatui_image::picker::Picker::from_query_stdio`] (or similar)
    /// **before** entering the alternate screen and pass the result here.
    pub fn with_picker(picker: ratatui_image::picker::Picker) -> Self {
        Self {
            cached_path: None,
            content: PreviewContent::Empty,
            scroll: 0,
            cached_width: 0,
            cached_height: 0,
            picker,
        }
    }

    /// Return the detected image protocol type (for diagnostics / UI display).
    pub fn protocol_type(&self) -> ratatui_image::picker::ProtocolType {
        self.picker.protocol_type()
    }

    /// Reload the preview if the path changed or the available area was
    /// resized.  When neither changed, this is a no-op.
    pub fn update(&mut self, path: Option<&Path>, area_width: u16, area_height: u16) {
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

        match path {
            Some(p) => {
                self.cached_path = Some(p.to_path_buf());
                self.content = load_preview(p, area_width, area_height, &mut self.picker);
            }
            None => {
                self.cached_path = None;
                self.content = PreviewContent::Empty;
            }
        }
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
        self.cached_path = None;
        self.cached_width = 0;
        self.cached_height = 0;
    }
}

// ── Loading helpers (private) ─────────────────────────────────────────────────

/// Top-level dispatcher: decide which loader to call based on metadata and
/// extension.
fn load_preview(
    path: &Path,
    _area_width: u16,
    _area_height: u16,
    picker: &mut ratatui_image::picker::Picker,
) -> PreviewContent {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return PreviewContent::Error(format!("{e}")),
    };

    if meta.is_dir() {
        return load_dir_preview(path);
    }

    // Check for image extension.
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if is_image_extension(ext) {
            return load_image_preview(path, picker);
        }
    }

    // Guard against very large files.
    if meta.len() > MAX_PREVIEW_BYTES as u64 {
        return PreviewContent::TooLarge { size: meta.len() };
    }

    load_text_preview(path)
}

/// Read a UTF-8 text file, capping at [`MAX_PREVIEW_LINES`].  Falls back to
/// binary preview if null bytes are detected in the first chunk.
fn load_text_preview(path: &Path) -> PreviewContent {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return PreviewContent::Error(format!("{e}")),
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
        Err(e) => return PreviewContent::Error(format!("{e}")),
    }

    // Re-open from the start (fill_buf does not consume, but to be safe with
    // large files we re-open).
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return PreviewContent::Error(format!("{e}")),
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

    PreviewContent::Text(TextPreview { lines, total_lines })
}

/// Decode a raster image and create a stateful protocol for rendering.
fn load_image_preview(path: &Path, picker: &mut ratatui_image::picker::Picker) -> PreviewContent {
    let dyn_img = match image::open(path) {
        Ok(i) => i,
        Err(e) => return PreviewContent::Error(format!("image decode: {e}")),
    };

    let protocol = picker.new_resize_protocol(dyn_img);
    PreviewContent::Image(Box::new(ImagePreview {
        protocol: std::cell::RefCell::new(protocol),
    }))
}

/// Read the first [`BINARY_DUMP_BYTES`] and format a classic hex dump.
fn load_binary_preview(path: &Path) -> PreviewContent {
    let total_bytes = match fs::metadata(path) {
        Ok(m) => m.len(),
        Err(e) => return PreviewContent::Error(format!("{e}")),
    };

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return PreviewContent::Error(format!("{e}")),
    };

    let mut buf = vec![0u8; BINARY_DUMP_BYTES];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(e) => return PreviewContent::Error(format!("{e}")),
    };
    buf.truncate(n);

    let hex_lines = format_hex_dump(&buf);

    PreviewContent::Binary(BinaryPreview {
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
fn load_dir_preview(path: &Path) -> PreviewContent {
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => return PreviewContent::Error(format!("{e}")),
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

    PreviewContent::Directory(DirPreview {
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
        Some(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| p.display().to_string()),
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
        PreviewContent::Empty => render_info(frame, content_area, "Nothing selected", theme),
        PreviewContent::TooLarge { size } => {
            let msg = format!("File too large for preview ({} bytes)", size);
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
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    // ── Helpers ───────────────────────────────────────────────────────────

    fn make_terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 24)).unwrap()
    }

    fn default_theme() -> Theme {
        Theme::default()
    }

    /// Create a tiny 2×2 RGBA PNG in memory for image tests.
    fn make_tiny_png() -> Vec<u8> {
        let mut img = image::RgbImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        img.put_pixel(1, 0, image::Rgb([0, 255, 0]));
        img.put_pixel(0, 1, image::Rgb([0, 0, 255]));
        img.put_pixel(1, 1, image::Rgb([255, 255, 0]));
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        use image::ImageEncoder;
        encoder
            .write_image(&img, 2, 2, image::ExtendedColorType::Rgb8)
            .unwrap();
        buf
    }

    /// Create a tiny 2×2 WebP image in memory for webp tests.
    fn make_tiny_webp() -> Vec<u8> {
        let img = image::RgbImage::from_fn(2, 2, |x, y| match (x, y) {
            (0, 0) => image::Rgb([255, 0, 0]),
            (1, 0) => image::Rgb([0, 255, 0]),
            (0, 1) => image::Rgb([0, 0, 255]),
            _ => image::Rgb([255, 255, 0]),
        });
        let mut buf = Vec::new();
        let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
        use image::ImageEncoder;
        encoder
            .write_image(&img, 2, 2, image::ExtendedColorType::Rgb8)
            .unwrap();
        buf
    }

    // ── File-type detection ───────────────────────────────────────────────

    #[test]
    fn is_image_extension_webp() {
        assert!(is_image_extension("webp"));
    }

    #[test]
    fn is_image_extension_png() {
        assert!(is_image_extension("png"));
    }

    #[test]
    fn is_image_extension_jpg() {
        assert!(is_image_extension("jpg"));
    }

    #[test]
    fn is_image_extension_jpeg() {
        assert!(is_image_extension("jpeg"));
    }

    #[test]
    fn is_image_extension_case_insensitive() {
        assert!(is_image_extension("PNG"));
        assert!(is_image_extension("Jpg"));
        assert!(is_image_extension("WEBP"));
    }

    #[test]
    fn is_image_extension_unknown_returns_false() {
        assert!(!is_image_extension("rs"));
        assert!(!is_image_extension("txt"));
        assert!(!is_image_extension(""));
    }

    #[test]
    fn is_likely_binary_detects_null_bytes() {
        let data = b"hello\x00world";
        assert!(is_likely_binary(data));
    }

    #[test]
    fn is_likely_binary_text_returns_false() {
        let data = b"just some plain text\n";
        assert!(!is_likely_binary(data));
    }

    // ── PreviewState ──────────────────────────────────────────────────────

    #[test]
    fn preview_state_new_has_empty_content() {
        let state = PreviewState::new();
        assert!(state.cached_path.is_none());
        assert!(matches!(state.content, PreviewContent::Empty));
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn preview_state_invalidate_clears_cache() {
        let mut state = PreviewState::new();
        state.cached_path = Some(PathBuf::from("/tmp/dummy"));
        state.cached_width = 80;
        state.cached_height = 24;
        state.invalidate();
        assert!(state.cached_path.is_none());
        assert_eq!(state.cached_width, 0);
        assert_eq!(state.cached_height, 0);
    }

    #[test]
    fn preview_state_scroll_up_clamps_at_zero() {
        let mut state = PreviewState::new();
        state.scroll = 3;
        state.scroll_up(5);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn preview_state_scroll_down_increments() {
        let mut state = PreviewState::new();
        state.scroll_down(7);
        assert_eq!(state.scroll, 7);
        state.scroll_down(3);
        assert_eq!(state.scroll, 10);
    }

    // ── TextPreview ───────────────────────────────────────────────────────

    #[test]
    fn load_text_preview_reads_file_content() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("hello.txt");
        fs::write(&file_path, "line one\nline two\nline three\n").unwrap();

        match load_text_preview(&file_path) {
            PreviewContent::Text(tp) => {
                assert_eq!(tp.lines.len(), 3);
                assert_eq!(tp.lines[0], "line one");
                assert_eq!(tp.lines[2], "line three");
                assert_eq!(tp.total_lines, 3);
            }
            other => panic!("expected Text, got {:?}", content_variant(&other)),
        }
    }

    #[test]
    fn load_text_preview_limits_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("big.txt");
        let content: String = (0..MAX_PREVIEW_LINES + 500)
            .map(|i| format!("line {i}\n"))
            .collect();
        fs::write(&file_path, &content).unwrap();

        match load_text_preview(&file_path) {
            PreviewContent::Text(tp) => {
                assert_eq!(tp.lines.len(), MAX_PREVIEW_LINES);
                assert!(tp.total_lines >= MAX_PREVIEW_LINES + 500);
            }
            other => panic!("expected Text, got {:?}", content_variant(&other)),
        }
    }

    #[test]
    fn load_text_preview_nonexistent_returns_error() {
        let result = load_text_preview(Path::new("/nonexistent/path/file.txt"));
        assert!(
            matches!(result, PreviewContent::Error(_)),
            "expected Error variant"
        );
    }

    // ── BinaryPreview ─────────────────────────────────────────────────────

    #[test]
    fn load_binary_preview_formats_hex_lines() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("data.bin");
        let data: Vec<u8> = (0u8..=255).collect();
        fs::write(&file_path, &data).unwrap();

        match load_binary_preview(&file_path) {
            PreviewContent::Binary(bp) => {
                assert!(!bp.hex_lines.is_empty());
                assert_eq!(bp.total_bytes, 256);
                // First line should start with "00000000"
                assert!(bp.hex_lines[0].starts_with("00000000"));
            }
            other => panic!("expected Binary, got {:?}", content_variant(&other)),
        }
    }

    // ── DirPreview ────────────────────────────────────────────────────────

    #[test]
    fn load_dir_preview_counts_entries() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "aaa").unwrap();
        fs::write(dir.path().join("b.txt"), "bbb").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        match load_dir_preview(dir.path()) {
            PreviewContent::Directory(dp) => {
                assert_eq!(dp.entry_count, 3);
                assert_eq!(dp.file_count, 2);
                assert_eq!(dp.dir_count, 1);
                assert!(dp.total_size > 0);
            }
            other => panic!("expected Directory, got {:?}", content_variant(&other)),
        }
    }

    // ── ImagePreview ──────────────────────────────────────────────────────

    #[test]
    fn load_image_preview_decodes_and_resizes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let png_path = dir.path().join("tiny.png");
        std::fs::write(&png_path, make_tiny_png()).expect("write png");

        let mut picker = ratatui_image::picker::Picker::halfblocks();
        let content = load_image_preview(&png_path, &mut picker);
        assert!(
            matches!(content, PreviewContent::Image(_)),
            "expected Image variant"
        );
    }

    #[test]
    fn load_image_preview_decodes_webp() {
        let dir = tempfile::tempdir().expect("tempdir");
        let webp_path = dir.path().join("tiny.webp");
        std::fs::write(&webp_path, make_tiny_webp()).expect("write webp");

        let mut picker = ratatui_image::picker::Picker::halfblocks();
        let content = load_image_preview(&webp_path, &mut picker);
        assert!(
            matches!(content, PreviewContent::Image(_)),
            "expected Image variant for webp, got {}",
            content_variant(&content)
        );
    }

    #[test]
    fn webp_preview_via_update() {
        let dir = tempfile::tempdir().expect("tempdir");
        let webp_path = dir.path().join("photo.webp");
        std::fs::write(&webp_path, make_tiny_webp()).expect("write webp");

        let mut state = PreviewState::new();
        state.update(Some(&webp_path), 80, 24);

        assert!(
            matches!(state.content, PreviewContent::Image(_)),
            "expected Image via update(), got {}",
            content_variant(&state.content)
        );
    }

    #[test]
    fn render_preview_webp_does_not_panic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let webp_path = dir.path().join("tiny.webp");
        std::fs::write(&webp_path, make_tiny_webp()).expect("write webp");

        let mut picker = ratatui_image::picker::Picker::halfblocks();
        let content = load_image_preview(&webp_path, &mut picker);

        let state = PreviewState {
            cached_path: Some(webp_path),
            content,
            scroll: 0,
            cached_width: 80,
            cached_height: 24,
            picker,
        };

        let mut terminal = make_terminal();
        let theme = default_theme();
        terminal
            .draw(|frame| render_preview(frame, frame.area(), &state, &theme))
            .expect("draw");
    }

    // ── Rendering smoke tests ─────────────────────────────────────────────

    #[test]
    fn render_preview_text_does_not_panic() {
        let mut terminal = make_terminal();
        let theme = default_theme();
        let state = PreviewState {
            cached_path: Some(PathBuf::from("test.rs")),
            content: PreviewContent::Text(TextPreview {
                lines: vec!["fn main() {}".to_string(), "// done".to_string()],
                total_lines: 2,
            }),
            scroll: 0,
            cached_width: 80,
            cached_height: 24,
            picker: ratatui_image::picker::Picker::halfblocks(),
        };
        terminal
            .draw(|frame| render_preview(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn render_preview_empty_does_not_panic() {
        let mut terminal = make_terminal();
        let theme = default_theme();
        let state = PreviewState::new();
        terminal
            .draw(|frame| render_preview(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn render_preview_error_does_not_panic() {
        let mut terminal = make_terminal();
        let theme = default_theme();
        let state = PreviewState {
            cached_path: None,
            content: PreviewContent::Error("something broke".to_string()),
            scroll: 0,
            cached_width: 80,
            cached_height: 24,
            picker: ratatui_image::picker::Picker::halfblocks(),
        };
        terminal
            .draw(|frame| render_preview(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn render_preview_binary_does_not_panic() {
        let mut terminal = make_terminal();
        let theme = default_theme();
        let state = PreviewState {
            cached_path: Some(PathBuf::from("data.bin")),
            content: PreviewContent::Binary(BinaryPreview {
                hex_lines: vec![
                    "00000000  00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f  |................|"
                        .to_string(),
                ],
                total_bytes: 16,
            }),
            scroll: 0,
            cached_width: 80,
            cached_height: 24,
            picker: ratatui_image::picker::Picker::halfblocks(),
        };
        terminal
            .draw(|frame| render_preview(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn render_preview_directory_does_not_panic() {
        let mut terminal = make_terminal();
        let theme = default_theme();
        let state = PreviewState {
            cached_path: Some(PathBuf::from("/tmp")),
            content: PreviewContent::Directory(DirPreview {
                entry_count: 5,
                file_count: 3,
                dir_count: 2,
                total_size: 12345,
            }),
            scroll: 0,
            cached_width: 80,
            cached_height: 24,
            picker: ratatui_image::picker::Picker::halfblocks(),
        };
        terminal
            .draw(|frame| render_preview(frame, frame.area(), &state, &theme))
            .unwrap();
    }

    #[test]
    fn render_preview_image_does_not_panic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let png_path = dir.path().join("tiny.png");
        std::fs::write(&png_path, make_tiny_png()).expect("write png");

        let mut picker = ratatui_image::picker::Picker::halfblocks();
        let content = load_image_preview(&png_path, &mut picker);

        let state = PreviewState {
            cached_path: Some(png_path),
            content,
            scroll: 0,
            cached_width: 80,
            cached_height: 24,
            picker,
        };

        let mut terminal = make_terminal();
        let theme = default_theme();
        terminal
            .draw(|frame| render_preview(frame, frame.area(), &state, &theme))
            .expect("draw");
    }

    // ── Lazy picker / no-query tests ──────────────────────────────────────
    //
    // These tests verify that previews work correctly with the default
    // halfblocks picker (i.e. without calling `from_query_stdio`).  This is
    // the code path taken when stdout is piped and the eager protocol query
    // is skipped to avoid a 2-second timeout and a raw-mode race condition.

    #[test]
    fn text_preview_works_with_halfblocks_picker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("hello.txt");
        std::fs::write(&file, "line one\nline two\n").unwrap();

        let mut state = PreviewState::new(); // halfblocks, no query
        state.update(Some(&file), 80, 24);

        assert!(
            matches!(state.content, PreviewContent::Text(_)),
            "expected Text, got {}",
            content_variant(&state.content)
        );
    }

    #[test]
    fn binary_preview_works_with_halfblocks_picker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("blob.bin");
        std::fs::write(&file, b"\x00\x01\x02\x03\x04").unwrap();

        let mut state = PreviewState::new();
        state.update(Some(&file), 80, 24);

        assert!(
            matches!(state.content, PreviewContent::Binary(_)),
            "expected Binary, got {}",
            content_variant(&state.content)
        );
    }

    #[test]
    fn directory_preview_works_with_halfblocks_picker() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), b"a").unwrap();

        let mut state = PreviewState::new();
        state.update(Some(dir.path()), 80, 24);

        assert!(
            matches!(state.content, PreviewContent::Directory(_)),
            "expected Directory, got {}",
            content_variant(&state.content)
        );
    }

    #[test]
    fn image_preview_works_with_halfblocks_picker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let png_path = dir.path().join("pic.png");
        std::fs::write(&png_path, make_tiny_png()).unwrap();

        let mut state = PreviewState::new(); // halfblocks — no from_query_stdio
        state.update(Some(&png_path), 80, 24);

        assert!(
            matches!(state.content, PreviewContent::Image(_)),
            "expected Image, got {}",
            content_variant(&state.content)
        );
    }

    #[test]
    fn preview_state_default_uses_halfblocks() {
        let state = PreviewState::new();
        assert_eq!(
            state.protocol_type(),
            ratatui_image::picker::ProtocolType::Halfblocks,
        );
    }

    // ── Test utility ──────────────────────────────────────────────────────

    /// Return a printable label for a [`PreviewContent`] variant (for error
    /// messages in assertions).
    fn content_variant(c: &PreviewContent) -> &'static str {
        match c {
            PreviewContent::Text(_) => "Text",
            PreviewContent::Image(_) => "Image",
            PreviewContent::Binary(_) => "Binary",
            PreviewContent::Directory(_) => "Directory",
            PreviewContent::Empty => "Empty",
            PreviewContent::TooLarge { .. } => "TooLarge",
            PreviewContent::Error(_) => "Error",
        }
    }
}
