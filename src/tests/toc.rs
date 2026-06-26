use super::{test_assets, test_md_theme};
use crate::app::App;
use crate::markdown::parse_markdown;
use crate::*;
use ratatui::layout::Rect;

fn toc(entries: &[(u8, usize)]) -> Vec<TocEntry> {
    entries
        .iter()
        .enumerate()
        .map(|(i, (level, line))| TocEntry {
            level: *level,
            title: format!("Section {}", i + 1),
            line: *line,
        })
        .collect()
}

fn make_app_with_toc(total_lines: usize, viewport_height: u16, toc: Vec<TocEntry>) -> App {
    let (ss, theme) = test_assets();
    let md = (0..total_lines)
        .map(|_| "line")
        .collect::<Vec<_>>()
        .join("\n");
    let (lines, _, _, _) = parse_markdown(&md, &ss, &theme, &test_md_theme(), false).into();
    let mut app = App::new(lines, toc, "test".to_string(), false, false, None, None);
    app.content_area = Rect::new(0, 0, 80, viewport_height);
    app
}

#[test]
fn active_toc_highlights_last_header_when_short_section_at_bottom() {
    let mut app = make_app_with_toc(100, 15, toc(&[(2, 0), (2, 30), (2, 70), (2, 95)]));
    app.scroll_bottom();
    assert_eq!(app.active_toc_index(), Some(3));
}

#[test]
fn active_toc_unchanged_when_document_fits_in_viewport() {
    let mut app = make_app_with_toc(10, 20, toc(&[(2, 0), (2, 5)]));
    app.scroll_bottom();
    assert_eq!(app.active_toc_index(), Some(0));
}

#[test]
fn active_toc_last_header_with_long_section_uses_existing_logic() {
    let mut app = make_app_with_toc(100, 15, toc(&[(2, 0), (2, 30), (2, 50)]));
    app.scroll_bottom();
    assert_eq!(app.active_toc_index(), Some(2));
}

#[test]
fn active_toc_intermediate_header() {
    let mut app = make_app_with_toc(100, 15, toc(&[(2, 0), (2, 30), (2, 70)]));
    app.scroll = 40;
    assert_eq!(app.active_toc_index(), Some(1));
}

#[test]
fn active_toc_empty_toc_returns_none() {
    let app = make_app_with_toc(50, 15, vec![]);
    assert_eq!(app.active_toc_index(), None);
}

#[test]
fn active_toc_single_header() {
    let app = make_app_with_toc(50, 15, toc(&[(2, 0)]));
    assert_eq!(app.active_toc_index(), Some(0));
}

#[test]
fn toc_only_includes_first_two_heading_levels() {
    let (ss, theme) = test_assets();
    let (_, toc, _, _) = parse_markdown(
        "# One\n## Two\n### Three\n#### Four\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
    )
    .into();

    assert_eq!(toc.len(), 3);
    assert_eq!(toc[0].level, 1);
    assert_eq!(toc[1].level, 2);
    assert_eq!(toc[2].level, 3);
}

#[test]
fn frontmatter_is_ignored_in_toc() {
    let (ss, theme) = test_assets();
    let src = "---\ntitle: Demo\nowner: me\n---\n# Visible\nBody\n";
    let (_, toc, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false).into();

    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Visible");
}

#[test]
fn toc_hides_unique_top_and_promotes_when_shallow() {
    let toc = toc(&[(1, 0), (2, 10), (2, 20)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, None);
    assert_eq!(levels.display_level(1), None);
    assert_eq!(levels.display_level(2), Some(1));
}

#[test]
fn toc_hides_unique_top_and_shows_two_paliers() {
    let toc = toc(&[(1, 0), (2, 10), (3, 15)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(3));
    assert_eq!(levels.display_level(1), None);
    assert_eq!(levels.display_level(2), Some(1));
    assert_eq!(levels.display_level(3), Some(2));
}

#[test]
fn toc_keeps_single_heading_as_root() {
    let toc = toc(&[(1, 0)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 1);
    assert_eq!(levels.sub, None);
    assert_eq!(levels.display_level(1), Some(1));
}

#[test]
fn toc_keeps_non_unique_top_as_root() {
    let toc = toc(&[(2, 0), (2, 10), (3, 14)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(3));
}

#[test]
fn toc_promotes_unique_deep_root() {
    let toc = toc(&[(3, 0), (4, 5), (5, 10)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 4);
    assert_eq!(levels.sub, Some(5));
    assert_eq!(levels.display_level(3), None);
    assert_eq!(levels.display_level(4), Some(1));
    assert_eq!(levels.display_level(5), Some(2));
}

#[test]
fn toc_deep_non_unique_top_is_root() {
    let toc = toc(&[(3, 0), (3, 10), (4, 14)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 3);
    assert_eq!(levels.sub, Some(4));
}

#[test]
fn toc_promotion_is_not_recursive() {
    let toc = toc(&[(1, 0), (2, 5), (3, 8), (3, 12)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(3));
    assert_eq!(levels.display_level(1), None);
    assert_eq!(levels.display_level(2), Some(1));
    assert_eq!(levels.display_level(3), Some(2));
}

#[test]
fn toc_ignores_level_gaps_two_paliers() {
    let toc = toc(&[(1, 0), (3, 5), (3, 10)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 3);
    assert_eq!(levels.sub, None);
    assert_eq!(levels.display_level(1), None);
    assert_eq!(levels.display_level(3), Some(1));
}

#[test]
fn toc_ignores_level_gaps_three_paliers() {
    let toc = toc(&[(1, 0), (2, 5), (2, 9), (4, 12)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(4));
    assert_eq!(levels.display_level(2), Some(1));
    assert_eq!(levels.display_level(4), Some(2));
}

#[test]
fn toc_sub_is_next_present_palier() {
    let toc = toc(&[(2, 0), (2, 5), (4, 9)]);
    let levels = toc_levels(&toc).unwrap();
    assert_eq!(levels.root, 2);
    assert_eq!(levels.sub, Some(4));
}

#[test]
fn toc_levels_empty_returns_none() {
    assert!(toc_levels(&[]).is_none());
}

#[test]
fn normalize_keeps_top_three_paliers() {
    let toc = toc(&[(2, 0), (3, 5), (4, 10), (5, 15)]);
    let normalized = normalize_toc(toc);
    assert_eq!(
        normalized.iter().map(|e| e.level).collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
}
