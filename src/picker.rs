//! A filter-as-you-type list overlay with fuzzy matching.
//!
//! Used for bookmarks and for the recursive fuzzy file finder, but entirely
//! generic: build a [`Picker`] from [`PickerItem`]s, feed it key events and
//! draw it with [`render_picker`].

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::palette::Theme;

// ── Fuzzy matching ────────────────────────────────────────────────────────────

/// Score `candidate` against `query` as a case-insensitive subsequence match.
///
/// Returns `None` when `query` is not a subsequence of `candidate`, otherwise
/// `(score, matched_char_indices)`. Higher is better. Rewards: consecutive
/// matches, matches at the start of a path segment / word, an exact prefix,
/// and shorter candidates.
#[must_use]
pub fn fuzzy_match(query: &str, candidate: &str) -> Option<(i64, Vec<usize>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }
    let cand: Vec<char> = candidate.chars().collect();
    let lower: Vec<char> = candidate.chars().flat_map(char::to_lowercase).collect();
    // `to_lowercase` can change length for exotic characters; fall back to the
    // original characters in that (rare) case so indices stay aligned.
    let lower = if lower.len() == cand.len() {
        lower
    } else {
        cand.clone()
    };
    let q: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();

    let mut positions = Vec::with_capacity(q.len());
    let mut score: i64 = 0;
    let mut ci = 0usize;
    let mut prev: Option<usize> = None;

    for &qc in &q {
        let found = (ci..lower.len()).find(|&i| lower[i] == qc)?;
        score += 10;
        if prev == Some(found.wrapping_sub(1)) && found > 0 {
            score += 15; // consecutive
        }
        let boundary = found == 0
            || matches!(cand[found - 1], '/' | '\\' | '.' | '_' | '-' | ' ')
            || (cand[found - 1].is_lowercase() && cand[found].is_uppercase());
        if boundary {
            score += 12;
        }
        if let Some(p) = prev {
            score -= i64::try_from(found.saturating_sub(p + 1))
                .unwrap_or(0)
                .min(8);
        }
        positions.push(found);
        prev = Some(found);
        ci = found + 1;
    }
    if lower.starts_with(&q) {
        score += 25;
    }
    // Prefer matches in the file name over matches in the directory part.
    let name_start = cand.iter().rposition(|&c| c == '/').map_or(0, |i| i + 1);
    if positions.iter().all(|&p| p >= name_start) {
        score += 20;
    }
    score -= i64::try_from(cand.len()).unwrap_or(0) / 8;
    Some((score, positions))
}

// ── Picker state ──────────────────────────────────────────────────────────────

/// What choosing an item means to the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    /// Jump to a saved bookmark.
    Bookmarks,
    /// Jump to a file found by the recursive fuzzy finder.
    Fuzzy,
}

/// One selectable row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerItem {
    /// Text that is matched against the query and shown prominently.
    pub label: String,
    /// Secondary dim text (e.g. a containing directory).
    pub detail: String,
    /// The path this row stands for.
    pub path: PathBuf,
    /// Whether `path` is a directory.
    pub is_dir: bool,
}

/// Result of feeding a key to a [`Picker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerAction {
    /// Key consumed; keep the picker open.
    Pending,
    /// Close without choosing.
    Cancel,
    /// Choose this item.
    Choose(PickerItem),
    /// Remove the highlighted item from the underlying list (bookmarks).
    Delete(PickerItem),
}

/// Filterable list state.
#[derive(Debug, Clone)]
pub struct Picker {
    pub kind: PickerKind,
    pub title: String,
    pub query: String,
    items: Vec<PickerItem>,
    /// Indices into `items` that match `query`, best first, with the matched
    /// character positions inside the label (for highlighting).
    matches: Vec<(usize, Vec<usize>)>,
    /// Highlighted row within `matches`.
    pub cursor: usize,
    /// `true` while a background walk is still adding items.
    pub loading: bool,
    /// `true` when the walk stopped early (size/time cap).
    pub truncated: bool,
}

/// Cap on rows rendered/kept matching, so huge trees stay snappy.
const MAX_MATCHES: usize = 500;

impl Picker {
    /// A picker over `items`.
    #[must_use]
    pub fn new(kind: PickerKind, title: impl Into<String>, items: Vec<PickerItem>) -> Self {
        let mut p = Self {
            kind,
            title: title.into(),
            query: String::new(),
            items,
            matches: Vec::new(),
            cursor: 0,
            loading: false,
            truncated: false,
        };
        p.refilter(None);
        p
    }

    /// Append more items (from a background walk) and re-run the filter.
    pub fn extend(&mut self, more: impl IntoIterator<Item = PickerItem>) {
        let keep = self.selected().map(|i| i.path.clone());
        self.items.extend(more);
        self.refilter(keep);
    }

    /// Total number of candidate items.
    #[must_use]
    pub fn total(&self) -> usize {
        self.items.len()
    }

    /// Number of rows matching the current query.
    #[must_use]
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// The highlighted item, if any row matches.
    #[must_use]
    pub fn selected(&self) -> Option<&PickerItem> {
        self.matches.get(self.cursor).map(|(i, _)| &self.items[*i])
    }

    /// Visible rows in order, with their matched label positions.
    pub fn rows(&self) -> impl Iterator<Item = (&PickerItem, &[usize])> {
        self.matches
            .iter()
            .map(|(i, pos)| (&self.items[*i], pos.as_slice()))
    }

    /// Remove `item` from the candidates.
    pub fn remove(&mut self, item: &PickerItem) {
        let keep = self.selected().map(|i| i.path.clone());
        self.items.retain(|i| i != item);
        self.refilter(keep);
    }

    /// Re-run the filter, keeping the row for `keep` highlighted if it
    /// survives.
    fn refilter(&mut self, keep: Option<PathBuf>) {
        let selected_path = keep;
        let mut scored: Vec<(i64, usize, Vec<usize>)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                fuzzy_match(&self.query, &item.label).map(|(s, pos)| (s, idx, pos))
            })
            .collect();
        if self.query.is_empty() {
            // Keep source order (bookmarks: insertion order; walk: discovery).
        } else {
            // Best score first; ties keep source order (stable sort).
            scored.sort_by_key(|s| std::cmp::Reverse(s.0));
        }
        scored.truncate(MAX_MATCHES);
        self.matches = scored.into_iter().map(|(_, i, p)| (i, p)).collect();
        // Keep the same row highlighted if it survived the refilter.
        self.cursor = selected_path
            .and_then(|p| {
                self.matches
                    .iter()
                    .position(|(i, _)| self.items[*i].path == p)
            })
            .unwrap_or(0)
            .min(self.matches.len().saturating_sub(1));
    }

    /// Feed one key press.
    pub fn handle_key(&mut self, key: KeyEvent) -> PickerAction {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => PickerAction::Cancel,
            KeyCode::Char('c') if ctrl => PickerAction::Cancel,
            KeyCode::Enter => self
                .selected()
                .cloned()
                .map_or(PickerAction::Pending, PickerAction::Choose),
            KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                PickerAction::Pending
            }
            KeyCode::Char('p' | 'k') if ctrl => {
                self.cursor = self.cursor.saturating_sub(1);
                PickerAction::Pending
            }
            KeyCode::Down => {
                self.step_down(1);
                PickerAction::Pending
            }
            KeyCode::Char('n' | 'j') if ctrl => {
                self.step_down(1);
                PickerAction::Pending
            }
            KeyCode::PageUp => {
                self.cursor = self.cursor.saturating_sub(10);
                PickerAction::Pending
            }
            KeyCode::PageDown => {
                self.step_down(10);
                PickerAction::Pending
            }
            KeyCode::Delete if self.kind == PickerKind::Bookmarks => self
                .selected()
                .cloned()
                .map_or(PickerAction::Pending, PickerAction::Delete),
            KeyCode::Backspace => {
                self.query.pop();
                let keep = self.selected().map(|i| i.path.clone());
                self.refilter(keep);
                PickerAction::Pending
            }
            KeyCode::Char('u') if ctrl => {
                self.query.clear();
                let keep = self.selected().map(|i| i.path.clone());
                self.refilter(keep);
                PickerAction::Pending
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.query.push(c);
                self.refilter(None);
                PickerAction::Pending
            }
            _ => PickerAction::Pending,
        }
    }

    fn step_down(&mut self, n: usize) {
        let last = self.matches.len().saturating_sub(1);
        self.cursor = (self.cursor + n).min(last);
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Draw `picker` as a centred overlay over `area`.
pub fn render_picker(frame: &mut Frame, area: Rect, picker: &Picker, theme: &Theme) {
    if area.width < 20 || area.height < 6 {
        return;
    }
    let width = (area.width * 7 / 10).clamp(40.min(area.width), area.width);
    let height = (area.height * 7 / 10).clamp(8.min(area.height), area.height);
    let rect = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, rect);

    let status = if picker.loading {
        format!(" {}/{} …", picker.match_count(), picker.total())
    } else if picker.truncated {
        format!(" {}/{}+ (capped)", picker.match_count(), picker.total())
    } else {
        format!(" {}/{}", picker.match_count(), picker.total())
    };
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", picker.title),
            Style::default()
                .fg(theme.brand)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(vec![
            Span::styled(
                " Enter",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" open  ", Style::default().fg(theme.dim)),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" cancel ", Style::default().fg(theme.dim)),
            Span::styled(status, Style::default().fg(theme.dim)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    if inner.height < 2 {
        return;
    }

    // Query line.
    let query_line = Line::from(vec![
        Span::styled(
            "› ",
            Style::default()
                .fg(theme.brand)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            picker.query.as_str(),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2588}", Style::default().fg(theme.accent)),
    ]);
    frame.render_widget(
        Paragraph::new(query_line),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );

    // Result list.
    let list_area = Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1);
    let visible = usize::from(list_area.height);
    // Keep the cursor inside the window.
    let offset = picker.cursor.saturating_sub(visible.saturating_sub(1));
    let items: Vec<ListItem> = picker
        .rows()
        .skip(offset)
        .take(visible)
        .map(|(item, positions)| {
            let name_style = Style::default()
                .fg(if item.is_dir {
                    theme.dir
                } else {
                    theme.match_file
                })
                .add_modifier(Modifier::BOLD);
            let hit_style = Style::default()
                .fg(theme.brand)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            let mut spans: Vec<Span> = Vec::new();
            spans.push(Span::raw(if item.is_dir { "\u{1F4C1} " } else { "   " }));
            for (i, ch) in item.label.chars().enumerate() {
                let style = if positions.contains(&i) {
                    hit_style
                } else {
                    name_style
                };
                spans.push(Span::styled(ch.to_string(), style));
            }
            if !item.detail.is_empty() {
                spans.push(Span::styled(
                    format!("  {}", item.detail),
                    Style::default().fg(theme.dim),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    if items.is_empty() {
        let msg = if picker.loading {
            "searching…"
        } else {
            "no matches"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(msg, Style::default().fg(theme.dim)))
                .alignment(Alignment::Center),
            list_area,
        );
        return;
    }
    let mut state = ListState::default();
    state.select(Some(picker.cursor - offset));
    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme.sel_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(list, list_area, &mut state);
}

#[cfg(test)]
mod tests;
