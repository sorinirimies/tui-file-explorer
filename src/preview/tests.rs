//! Unit tests for [`super`].

use super::*;
use ratatui::{backend::TestBackend, Terminal};

// ── Helpers ───────────────────────────────────────────────────────────

fn halfblocks() -> ratatui_image::picker::Picker {
    ratatui_image::picker::Picker::halfblocks()
}

// Test-local wrappers: the production loaders return the thread-safe
// `Loaded`; these convert to the `PreviewContent` the assertions inspect.
fn load_text_preview(p: &Path) -> PreviewContent {
    super::load_text_preview(p).into_content(&mut halfblocks())
}
fn load_binary_preview(p: &Path) -> PreviewContent {
    super::load_binary_preview(p).into_content(&mut halfblocks())
}
fn load_dir_preview(p: &Path) -> PreviewContent {
    super::load_dir_preview(p).into_content(&mut halfblocks())
}
fn load_image_preview(p: &Path, picker: &mut ratatui_image::picker::Picker) -> PreviewContent {
    super::load_image_preview(p).into_content(picker)
}

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
    state.wait_until_loaded(std::time::Duration::from_secs(5));

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
        task: None,
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
        task: None,
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
        task: None,
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
        task: None,
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
        task: None,
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
        task: None,
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
    state.wait_until_loaded(std::time::Duration::from_secs(5));

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
    state.wait_until_loaded(std::time::Duration::from_secs(5));

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
    state.wait_until_loaded(std::time::Duration::from_secs(5));

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
    state.wait_until_loaded(std::time::Duration::from_secs(5));

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
        PreviewContent::Archive(_) => "Archive",
        PreviewContent::Loading => "Loading",
        PreviewContent::Empty => "Empty",
        PreviewContent::TooLarge { .. } => "TooLarge",
        PreviewContent::Error(_) => "Error",
    }
}

// ── Async loading & archives ──────────────────────────────────────────

#[test]
fn update_shows_loading_then_final_content() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "hello\nworld\n").unwrap();
    let mut state = PreviewState::new();
    state.update(Some(&file), 80, 24);
    // Immediately after the request the load is in flight (or already done).
    assert!(state.is_loading() || matches!(state.content, PreviewContent::Text(_)));
    state.wait_until_loaded(std::time::Duration::from_secs(5));
    assert!(!state.is_loading());
    assert!(matches!(&state.content, PreviewContent::Text(t) if t.total_lines == 2));
}

#[test]
fn switching_paths_discards_the_stale_result() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    std::fs::write(&a, "AAA").unwrap();
    std::fs::write(&b, "BBB\nBBB").unwrap();
    let mut state = PreviewState::new();
    state.update(Some(&a), 80, 24);
    state.update(Some(&b), 80, 24);
    state.wait_until_loaded(std::time::Duration::from_secs(5));
    assert_eq!(state.cached_path.as_deref(), Some(b.as_path()));
    assert!(matches!(&state.content, PreviewContent::Text(t) if t.total_lines == 2));
}

#[test]
fn update_with_no_path_clears_and_cancels() {
    let mut state = PreviewState::new();
    state.update(Some(Path::new("/definitely/not/here")), 80, 24);
    state.update(None, 80, 24);
    assert!(!state.is_loading());
    assert!(matches!(state.content, PreviewContent::Empty));
}

#[test]
fn loading_state_renders_a_placeholder() {
    let mut state = PreviewState::new();
    state.content = PreviewContent::Loading;
    let mut terminal = make_terminal();
    terminal
        .draw(|f| render_preview(f, f.area(), &state, &default_theme()))
        .unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("Loading"));
}

#[test]
fn archives_are_listed_not_dumped() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("pack.zip");
    {
        let mut w = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        w.start_file("inside/notes.txt", opts).unwrap();
        w.write_all(b"12345").unwrap();
        w.finish().unwrap();
    }
    let mut state = PreviewState::new();
    state.update(Some(&zip_path), 80, 24);
    state.wait_until_loaded(std::time::Duration::from_secs(5));
    assert!(matches!(&state.content, PreviewContent::Archive(a) if a.total_entries == 1));

    let mut terminal = make_terminal();
    terminal
        .draw(|f| render_preview(f, f.area(), &state, &default_theme()))
        .unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("inside/notes.txt"));
    assert!(text.contains("ZIP"));
}
