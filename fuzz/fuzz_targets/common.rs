#![allow(dead_code)]
use arbitrary::Arbitrary;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use std::io;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use tui_file_explorer::{
    DiskUsage, FileSystem, FsEntry, SharedFileSystem, SortMode, StdFileSystem, Theme,
};

/// Read-only filesystem so fuzzed keystrokes can never modify the host.
#[derive(Debug)]
pub struct ReadOnlyFs(StdFileSystem);

fn denied<T>() -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::PermissionDenied, "read-only"))
}

impl FileSystem for ReadOnlyFs {
    fn read_dir(&self, p: &Path) -> io::Result<Vec<FsEntry>> {
        self.0.read_dir(p)
    }
    fn create_dir_all(&self, _: &Path) -> io::Result<()> {
        denied()
    }
    fn create_file(&self, _: &Path) -> io::Result<()> {
        denied()
    }
    fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
        denied()
    }
    fn copy_file(&self, _: &Path, _: &Path) -> io::Result<()> {
        denied()
    }
    fn copy_dir(&self, _: &Path, _: &Path) -> io::Result<()> {
        denied()
    }
    fn remove_file(&self, _: &Path) -> io::Result<()> {
        denied()
    }
    fn remove_dir_all(&self, _: &Path) -> io::Result<()> {
        denied()
    }
    fn exists(&self, p: &Path) -> bool {
        self.0.exists(p)
    }
    fn is_dir(&self, p: &Path) -> bool {
        self.0.is_dir(p)
    }
    fn disk_usage(&self, p: &Path) -> Option<DiskUsage> {
        self.0.disk_usage(p)
    }
    fn dir_size(&self, p: &Path) -> (u64, bool) {
        self.0.dir_size(p)
    }
}

pub fn read_only_fs() -> SharedFileSystem {
    Arc::new(ReadOnlyFs(StdFileSystem))
}

/// A small fixed tree (odd names included) shared by all iterations.
pub fn fixture_dir() -> &'static Path {
    static DIR: OnceLock<std::path::PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let d = std::env::temp_dir().join(format!("tfe-fuzz-{}", std::process::id()));
        let _ = std::fs::create_dir_all(d.join("sub/inner"));
        let _ = std::fs::create_dir_all(d.join("日本語 dir"));
        for f in ["a.txt", ".hidden", "b.iso", "ü.rs", "sub/c.md", "sp ace.png"] {
            let _ = std::fs::write(d.join(f), b"fuzz");
        }
        d
    })
}

#[derive(Arbitrary, Debug, Clone, Copy)]
pub enum FuzzSort {
    Name,
    SizeDesc,
    Extension,
}

pub fn sort_mode(s: FuzzSort) -> SortMode {
    match s {
        FuzzSort::Name => SortMode::Name,
        FuzzSort::SizeDesc => SortMode::SizeDesc,
        FuzzSort::Extension => SortMode::Extension,
    }
}

#[derive(Arbitrary, Debug)]
pub struct FuzzKey {
    kind: u8,
    ch: char,
    mods: u8,
}

impl FuzzKey {
    pub fn event(&self) -> KeyEvent {
        let code = match self.kind % 20 {
            0 => KeyCode::Up,
            1 => KeyCode::Down,
            2 => KeyCode::Left,
            3 => KeyCode::Right,
            4 => KeyCode::Enter,
            5 => KeyCode::Esc,
            6 => KeyCode::Backspace,
            7 => KeyCode::Tab,
            8 => KeyCode::BackTab,
            9 => KeyCode::PageUp,
            10 => KeyCode::PageDown,
            11 => KeyCode::Home,
            12 => KeyCode::End,
            13 => KeyCode::Delete,
            14 => KeyCode::Insert,
            15 => KeyCode::F(self.mods),
            _ => KeyCode::Char(self.ch),
        };
        let mut m = KeyModifiers::NONE;
        if self.mods & 1 != 0 {
            m |= KeyModifiers::CONTROL;
        }
        if self.mods & 2 != 0 {
            m |= KeyModifiers::SHIFT;
        }
        if self.mods & 4 != 0 {
            m |= KeyModifiers::ALT;
        }
        let mut ev = KeyEvent::new(code, m);
        if self.mods & 0x80 != 0 {
            ev.kind = KeyEventKind::Release;
        }
        ev
    }
}

/// Bounded size (≤200x100, possibly 0) plus an inner area offset from origin.
#[derive(Arbitrary, Debug, Clone, Copy)]
pub struct FuzzArea {
    w: u8,
    h: u8,
    x: u8,
    y: u8,
    iw: u8,
    ih: u8,
}

impl FuzzArea {
    pub fn term_size(&self) -> (u16, u16) {
        (u16::from(self.w) % 201, u16::from(self.h) % 101)
    }
    pub fn terminal(&self) -> Terminal<TestBackend> {
        let (w, h) = self.term_size();
        Terminal::new(TestBackend::new(w, h)).unwrap()
    }
    /// Inner area; may extend beyond the buffer only via offset+size clamped by us.
    pub fn rect(&self) -> Rect {
        let (w, h) = self.term_size();
        let x = u16::from(self.x) % (w + 1);
        let y = u16::from(self.y) % (h + 1);
        let iw = u16::from(self.iw) % (w - x + 1);
        let ih = u16::from(self.ih) % (h - y + 1);
        Rect::new(x, y, iw, ih)
    }
}

pub fn theme() -> Theme {
    Theme::default()
}
