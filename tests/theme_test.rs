//! Integration tests for shared `tui-nuances` theming.
use ratatui::{backend::TestBackend, style::Color, Terminal};
use tui_file_explorer::{
    render_dual_pane_nuanced, render_dual_pane_themed, render_nuanced, render_themed, DualPane,
    FileExplorer, NuancesTheme, Theme, ThemeBuilder, ThemeProvider, Themed,
};

struct App;
impl ThemeProvider for App {
    fn nuance_theme(&self) -> NuancesTheme {
        NuancesTheme::nord()
    }
}

#[test]
fn default_unchanged_without_theme() {
    assert_eq!(ThemeBuilder::new().build(), Theme::default());
}

#[test]
fn provider_and_from_agree() {
    let n = NuancesTheme::gruvbox();
    assert_eq!(Theme::from(&n), Theme::from_nuances(&n));
    assert_eq!(
        Theme::from_provider(&App),
        Theme::from(&NuancesTheme::nord())
    );
    assert_eq!(Theme::default().themed(&App), Theme::from_provider(&App));
}

#[test]
fn explicit_wins_regardless_of_order() {
    let n = NuancesTheme::dracula();
    let a = ThemeBuilder::new().fg(Color::Red).theme(&n).build();
    let b = ThemeBuilder::new().theme(&n).fg(Color::Red).build();
    assert_eq!(a, b);
    assert_eq!(a.fg, Color::Red);
    assert_eq!(a.sel_bg, n.selection_bg);
}

#[test]
fn all_builtin_themes_render() {
    let dir = std::env::temp_dir();
    for n in NuancesTheme::all() {
        let mut term = Terminal::new(TestBackend::new(70, 16)).unwrap();
        let mut ex = FileExplorer::new(dir.clone(), vec![]);
        term.draw(|f| render_nuanced(&mut ex, f, f.area(), &n))
            .unwrap();
        term.draw(|f| render_themed(&mut ex, f, f.area(), &Theme::from(&n)))
            .unwrap();
        let mut dual = DualPane::builder(dir.clone()).build();
        term.draw(|f| render_dual_pane_nuanced(&mut dual, f, f.area(), &n))
            .unwrap();
        term.draw(|f| render_dual_pane_themed(&mut dual, f, f.area(), &Theme::from(&n)))
            .unwrap();
    }
}
