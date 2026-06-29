use super::{test_assets, test_md_theme};
use crate::app::App;
use crate::markdown::parse_markdown;
use crate::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
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

fn many_toc(count: usize) -> Vec<TocEntry> {
    (0..count)
        .map(|i| TocEntry {
            level: 2,
            title: format!("Section {}", i + 1),
            line: i * 10,
        })
        .collect()
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }
}

fn make_app_with_toc(total_lines: usize, viewport_height: u16, toc: Vec<TocEntry>) -> App {
    let (ss, theme) = test_assets();
    let md = (0..total_lines)
        .map(|_| "line")
        .collect::<Vec<_>>()
        .join("\n");
    let (lines, _, _, _) = parse_markdown(&md, &ss, &theme, &test_md_theme(), false, true).into();
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
        true,
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
    let (_, toc, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();

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

#[test]
fn toc_scroll_bounds_and_non_overflow_behavior() {
    let mut app = make_app_with_toc(200, 20, many_toc(12));
    app.refresh_toc_cache();

    assert!(app.toc_is_overflowing(5));
    assert_eq!(app.max_toc_scroll(5), 7);

    app.scroll_toc_down(100, 5);
    assert_eq!(app.toc_scroll(), 7);

    app.clamp_toc_scroll(10);
    assert_eq!(app.toc_scroll(), 2);

    app.scroll_toc_down(3, 20);
    assert_eq!(app.toc_scroll(), 0);
    assert!(!app.toc_is_overflowing(20));
}

#[test]
fn visible_toc_lines_start_at_toc_scroll() {
    let mut app = make_app_with_toc(200, 20, many_toc(12));
    app.refresh_toc_cache();
    app.scroll_toc_down(3, 5);

    let first = line_plain_text(&app.visible_toc_lines(5)[0]);
    assert!(first.contains("04"));
    assert!(first.contains("Section 4"));
}

#[test]
fn toc_hover_and_click_mapping_include_toc_scroll() {
    let mut app = make_app_with_toc(200, 20, many_toc(12));
    app.toc_list_area = Some(Rect::new(0, 0, 30, 5));
    app.refresh_toc_cache();
    app.scroll_toc_down(4, 5);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Moved, 5, 2)
    ));
    assert_eq!(app.hovered_toc_idx, Some(6));

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 5, 2)
    ));
    assert_eq!(app.scroll(), 60);
}

#[test]
fn mouse_wheel_over_toc_scrolls_toc_not_document() {
    let mut app = make_app_with_toc(200, 20, many_toc(12));
    app.toc_list_area = Some(Rect::new(0, 0, 30, 5));
    app.refresh_toc_cache();
    app.scroll_to(10);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::ScrollDown, 5, 1)
    ));

    assert_eq!(app.toc_scroll(), 3);
    assert_eq!(app.scroll(), 10);
}

#[test]
fn mouse_wheel_outside_toc_scrolls_document() {
    let mut app = make_app_with_toc(200, 20, many_toc(12));
    app.toc_list_area = Some(Rect::new(0, 0, 30, 5));
    app.refresh_toc_cache();
    app.scroll_to(10);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::ScrollDown, 40, 1)
    ));

    assert_eq!(app.toc_scroll(), 0);
    assert_eq!(app.scroll(), 13);
}

#[test]
fn active_toc_follow_keeps_active_heading_visible() {
    let mut app = make_app_with_toc(300, 20, many_toc(20));
    app.refresh_toc_cache();

    app.scroll_to(120);
    app.refresh_toc_cache();
    app.ensure_active_toc_visible(5);

    let active = app.active_toc_display_index().unwrap();
    assert!((app.toc_scroll()..app.toc_scroll() + 5).contains(&active));

    app.scroll_top();
    app.refresh_toc_cache();
    app.ensure_active_toc_visible(5);

    assert_eq!(app.active_toc_display_index(), Some(0));
    assert_eq!(app.toc_scroll(), 0);
}

#[test]
fn opening_toc_at_deep_scroll_positions_active_heading() {
    let mut app = make_app_with_toc(300, 20, many_toc(20));
    app.scroll_to(150);
    app.toggle_toc();
    app.refresh_toc_cache();
    app.ensure_active_toc_visible(5);

    let active = app.active_toc_display_index().unwrap();
    assert!((app.toc_scroll()..app.toc_scroll() + 5).contains(&active));
}

#[test]
fn manual_toc_scroll_is_not_overridden_by_active_follow() {
    let mut app = make_app_with_toc(300, 20, many_toc(20));
    app.refresh_toc_cache();
    app.scroll_to(120);
    app.refresh_toc_cache();
    app.ensure_active_toc_visible(5);
    assert!(app.toc_scroll() > 0);

    app.scroll_toc_up(100, 5);
    app.refresh_toc_cache();
    app.ensure_active_toc_visible(5);

    assert_eq!(app.toc_scroll(), 0);
}

#[test]
fn toc_scroll_clamps_after_resize_and_content_replace() {
    let mut app = make_app_with_toc(300, 20, many_toc(20));
    app.toc_list_area = Some(Rect::new(0, 0, 30, 5));
    app.refresh_toc_cache();
    app.scroll_toc_down(100, 5);
    assert_eq!(app.toc_scroll(), 15);

    app.clamp_toc_scroll(10);
    assert_eq!(app.toc_scroll(), 10);

    let (ss, theme) = test_assets();
    let parsed = parse_markdown(
        "# One\nbody\n# Two\nbody\n# Three\nbody\n",
        &ss,
        &theme,
        &test_md_theme(),
        false,
        true,
    );
    app.replace_content(parsed);

    assert_eq!(app.toc_scroll(), 0);
}

#[test]
fn toc_scrollbar_click_maps_row_to_toc_scroll() {
    let mut app = make_app_with_toc(300, 20, many_toc(20));
    app.toc_list_area = Some(Rect::new(0, 0, 30, 5));
    app.refresh_toc_cache();
    app.scroll_to(10);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 29, 4)
    ));

    assert_eq!(app.toc_scroll(), app.max_toc_scroll(5));
    assert_eq!(app.scroll(), 10);
    assert!(app.toc_scrollbar_dragging);
    assert!(!app.scrollbar_dragging);

    assert!(!handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Up(MouseButton::Left), 29, 4)
    ));
    assert!(!app.toc_scrollbar_dragging);
}
