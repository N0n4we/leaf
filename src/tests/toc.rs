use super::{test_assets, test_md_theme};
use crate::app::{App, AppConfig, DEFAULT_TOC_WIDTH, MIN_PREVIEW_WIDTH, MIN_TOC_WIDTH};
use crate::markdown::parse_markdown;
use crate::*;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};
use syntect::highlighting::ThemeSet;

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

fn render_app_buffer(app: &mut App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::render::ui(f, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn render_app(app: &mut App, width: u16, height: u16) {
    let _ = render_app_buffer(app, width, height);
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

#[test]
fn toc_split_width_defaults_and_clamps() {
    let app = make_app_with_toc(100, 10, many_toc(5));

    assert_eq!(app.toc_width(), DEFAULT_TOC_WIDTH);
    assert_eq!(
        App::clamp_toc_width_value(DEFAULT_TOC_WIDTH, 80),
        DEFAULT_TOC_WIDTH
    );
    assert_eq!(App::clamp_toc_width_value(100, 80), 50);
    assert_eq!(App::clamp_toc_width_value(1, 80), MIN_TOC_WIDTH);
    assert_eq!(App::clamp_toc_width_value(DEFAULT_TOC_WIDTH, 40), 10);
    assert_eq!(App::clamp_toc_width_value(DEFAULT_TOC_WIDTH, 0), 0);
    assert_eq!(80 - App::clamp_toc_width_value(100, 80), MIN_PREVIEW_WIDTH);
}

#[test]
fn toc_split_width_persists_across_hide_show() {
    let mut app = make_app_with_toc(100, 10, many_toc(5));
    app.toggle_toc();
    render_app(&mut app, 100, 12);

    app.start_toc_resizer_dragging();
    assert!(app.update_toc_width_from_column(39));
    app.stop_toc_resizer_dragging();
    assert_eq!(app.toc_width(), 40);

    app.toggle_toc();
    assert!(app.toc_resizer_area().is_none());
    assert!(!app.toc_resizer_dragging());
    assert_eq!(app.toc_width(), 40);

    app.toggle_toc();
    render_app(&mut app, 100, 12);
    assert_eq!(app.toc_width(), 40);
    assert_eq!(app.toc_resizer_area().unwrap().x, 39);
}

#[test]
fn rendered_toc_split_places_scrollbar_left_of_divider() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    render_app(&mut app, 100, 12);

    let divider = app.toc_resizer_area().unwrap();
    let toc_list = app.toc_list_area.unwrap();
    assert_eq!(app.toc_width(), DEFAULT_TOC_WIDTH);
    assert_eq!(divider, Rect::new(29, 0, 1, 11));
    assert_eq!(toc_list.width, 29);
    assert_eq!(toc_list.x + toc_list.width - 1, divider.x - 1);
    assert_eq!(app.content_area.x, 30);
    assert_eq!(app.content_area.width, 70);
}

#[test]
fn rendered_toc_header_right_border_aligns_with_divider() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    let buffer = render_app_buffer(&mut app, 100, 12);

    let divider = app.toc_resizer_area().unwrap();
    assert_eq!(buffer.cell((divider.x - 1, 1)).unwrap().symbol(), " ");
    assert_eq!(buffer.cell((divider.x, 1)).unwrap().symbol(), "│");

    app.start_toc_resizer_dragging();
    assert!(app.update_toc_width_from_column(39));
    app.stop_toc_resizer_dragging();
    let buffer = render_app_buffer(&mut app, 100, 12);

    let divider = app.toc_resizer_area().unwrap();
    assert_eq!(divider.x, 39);
    assert_eq!(buffer.cell((divider.x - 1, 1)).unwrap().symbol(), " ");
    assert_eq!(buffer.cell((divider.x, 1)).unwrap().symbol(), "│");
}

#[test]
fn rendered_toc_without_overflow_hides_scrollbar_track() {
    let mut app = make_app_with_toc(300, 10, many_toc(8));
    app.toggle_toc();
    let buffer = render_app_buffer(&mut app, 100, 12);

    let divider = app.toc_resizer_area().unwrap();
    let toc_list = app.toc_list_area.unwrap();
    assert!(!app.toc_is_overflowing(toc_list.height as usize));

    let scrollbar_col = divider.x - 1;
    for y in toc_list.y..toc_list.y + toc_list.height {
        assert_eq!(buffer.cell((scrollbar_col, y)).unwrap().symbol(), " ");
    }
}

#[test]
fn rendered_toc_scrollbar_track_uses_preview_scrollbar_color_rule() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    let buffer = render_app_buffer(&mut app, 100, 12);

    let toc_list = app.toc_list_area.unwrap();
    let toc_scrollbar_x = toc_list.x + toc_list.width - 1;
    let toc_track_y = toc_list.y + toc_list.height - 1;
    let preview_scrollbar_x = app.content_area.x + app.content_area.width - 1;
    let preview_track_y = app.content_area.y + app.content_area.height - 1;

    let toc_track = buffer.cell((toc_scrollbar_x, toc_track_y)).unwrap();
    let preview_track = buffer.cell((preview_scrollbar_x, preview_track_y)).unwrap();
    assert_eq!(toc_track.symbol(), "│");
    assert_eq!(preview_track.symbol(), "│");
    assert_eq!(toc_track.fg, preview_track.fg);
}

#[test]
fn rendered_toc_entries_keep_preview_sized_right_gutter() {
    let mut entries = many_toc(20);
    entries[0].title = "A heading long enough to fill the table of contents width".to_string();
    let mut app = make_app_with_toc(300, 10, entries);
    app.toggle_toc();
    let buffer = render_app_buffer(&mut app, 100, 12);

    let divider = app.toc_resizer_area().unwrap();
    let toc_list = app.toc_list_area.unwrap();
    let first_line = &app.visible_toc_lines(toc_list.height as usize)[0];

    assert_eq!(first_line.width() as u16, toc_list.width.saturating_sub(2));
    assert_eq!(
        buffer.cell((divider.x - 2, toc_list.y)).unwrap().symbol(),
        " "
    );
    assert_eq!(
        buffer.cell((divider.x - 1, toc_list.y)).unwrap().symbol(),
        "█"
    );
}

#[test]
fn rendered_dynamic_toc_split_moves_divider_and_preview() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    render_app(&mut app, 100, 12);

    app.start_toc_resizer_dragging();
    assert!(app.update_toc_width_from_column(39));
    app.stop_toc_resizer_dragging();
    render_app(&mut app, 100, 12);

    let divider = app.toc_resizer_area().unwrap();
    let toc_list = app.toc_list_area.unwrap();
    assert_eq!(app.toc_width(), 40);
    assert_eq!(divider.x, 39);
    assert_eq!(toc_list.width, 39);
    assert_eq!(toc_list.x + toc_list.width - 1, 38);
    assert_eq!(app.content_area.x, 40);
    assert_eq!(app.content_area.width, 60);
}

#[test]
fn sync_render_width_uses_toc_split_width_once() {
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let source =
        "# Heading\n\nA paragraph with enough words to make render width changes observable.";
    let (lines, toc, _, _) = crate::markdown::parse_markdown_with_width(
        source,
        &ss,
        &theme,
        80,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: source.to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    app.toggle_toc();

    assert!(sync_render_width_for_app(100, &mut app, &ss, &ts));
    assert_eq!(app.render_width(), 67);

    app.set_toc_split_area(Some(Rect::new(0, 0, 100, 10)));
    assert!(app.update_toc_width_from_column(39));
    assert!(sync_render_width_for_app(100, &mut app, &ss, &ts));
    assert_eq!(app.render_width(), 57);

    assert!(sync_render_width_for_app(10, &mut app, &ss, &ts));
    assert_eq!(app.toc_width(), 40);
    assert!(sync_render_width_for_app(100, &mut app, &ss, &ts));
    assert_eq!(app.toc_width(), 40);
    assert_eq!(app.render_width(), 57);
}

#[test]
fn sync_render_width_hidden_toc_uses_full_width() {
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let source =
        "# Heading\n\nA paragraph with enough words to make render width changes observable.";
    let (lines, toc, _, _) = crate::markdown::parse_markdown_with_width(
        source,
        &ss,
        &theme,
        80,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source: source.to_string(),
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );

    assert!(sync_render_width_for_app(100, &mut app, &ss, &ts));
    assert_eq!(app.render_width(), 97);
}

#[test]
fn mouse_drag_resizes_toc_split_and_release_stops_dragging() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    render_app(&mut app, 100, 12);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 29, 2)
    ));
    assert!(app.toc_resizer_dragging());
    assert_eq!(app.toc_width(), DEFAULT_TOC_WIDTH);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Drag(MouseButton::Left), 39, 2)
    ));
    assert_eq!(app.toc_width(), 40);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Up(MouseButton::Left), 39, 2)
    ));
    assert!(!app.toc_resizer_dragging());

    assert!(!handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Drag(MouseButton::Left), 49, 2)
    ));
    assert_eq!(app.toc_width(), 40);
}

#[test]
fn mouse_drag_beyond_bounds_clamps_and_unchanged_drag_does_not_reparse() {
    let (ss, theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let source = (0..20)
        .map(|idx| format!("## Section {idx}\nBody text that can reflow when widths change."))
        .collect::<Vec<_>>()
        .join("\n\n");
    let (lines, toc, _, _) = crate::markdown::parse_markdown_with_width(
        &source,
        &ss,
        &theme,
        80,
        &test_md_theme(),
        false,
        true,
    )
    .into();
    let mut app = App::new_with_source(
        lines,
        toc,
        AppConfig {
            filename: "stdin".to_string(),
            source,
            debug_input: false,
            watch: false,
            filepath: None,
            last_file_state: None,
        },
    );
    app.toggle_toc();
    render_app(&mut app, 80, 12);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 29, 2)
    ));
    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Drag(MouseButton::Left), 200, 2)
    ));
    assert_eq!(app.toc_width(), 50);
    assert!(sync_render_width_for_app(80, &mut app, &ss, &ts));
    let render_width = app.render_width();
    let total = app.total();

    assert!(!handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Drag(MouseButton::Left), 200, 2)
    ));
    assert!(!sync_render_width_for_app(80, &mut app, &ss, &ts));
    assert_eq!(app.render_width(), render_width);
    assert_eq!(app.total(), total);
}

#[test]
fn divider_drag_clears_scrollbar_drag_states() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    render_app(&mut app, 100, 12);
    app.toc_scrollbar_dragging = true;
    app.scrollbar_dragging = true;

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 29, 2)
    ));

    assert!(app.toc_resizer_dragging());
    assert!(!app.toc_scrollbar_dragging);
    assert!(!app.scrollbar_dragging);
}

#[test]
fn toc_scrollbar_and_divider_adjacent_columns_do_not_conflict() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    render_app(&mut app, 100, 12);
    app.scroll_to(10);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 28, 4)
    ));
    assert!(app.toc_scrollbar_dragging);
    assert!(!app.toc_resizer_dragging());
    assert_eq!(app.toc_width(), DEFAULT_TOC_WIDTH);
    assert_eq!(app.scroll(), 10);

    assert!(!handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Up(MouseButton::Left), 28, 4)
    ));

    let toc_scroll = app.toc_scroll();
    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 29, 4)
    ));
    assert!(app.toc_resizer_dragging());
    assert!(!app.toc_scrollbar_dragging);
    assert_eq!(app.toc_scroll(), toc_scroll);
    assert_eq!(app.scroll(), 10);
}

#[test]
fn divider_click_is_not_toc_entry_click() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    render_app(&mut app, 100, 12);
    app.scroll_to(10);

    assert!(handle_mouse_event(
        &mut app,
        mouse(MouseEventKind::Down(MouseButton::Left), 29, 5)
    ));

    assert_eq!(app.scroll(), 10);
    assert_eq!(app.hovered_toc_idx, None);
    assert!(app.toc_resizer_dragging());
}

#[test]
fn toc_right_padding_click_is_not_toc_entry_click() {
    let mut app = make_app_with_toc(300, 10, many_toc(20));
    app.toggle_toc();
    render_app(&mut app, 100, 12);
    app.scroll_to(10);

    let divider = app.toc_resizer_area().unwrap();
    let toc_list = app.toc_list_area.unwrap();

    assert!(!handle_mouse_event(
        &mut app,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider.x - 2,
            toc_list.y
        )
    ));

    assert_eq!(app.scroll(), 10);
    assert_eq!(app.hovered_toc_idx, None);
    assert!(!app.toc_scrollbar_dragging);
    assert!(!app.toc_resizer_dragging());
}

#[test]
fn wider_toc_split_rebuilds_toc_lines_with_more_title_text() {
    let mut entries = many_toc(3);
    entries[0].title = "A very long heading that should reveal more text when widened".to_string();
    let mut app = make_app_with_toc(100, 10, entries);
    app.toggle_toc();
    render_app(&mut app, 80, 12);
    let default_line = line_plain_text(&app.visible_toc_lines(8)[0]);

    app.start_toc_resizer_dragging();
    assert!(app.update_toc_width_from_column(49));
    app.stop_toc_resizer_dragging();
    render_app(&mut app, 100, 12);
    let wide_line = line_plain_text(&app.visible_toc_lines(8)[0]);

    assert!(wide_line.len() > default_line.len());
    assert!(wide_line.contains("heading"));
}

#[test]
fn secondary_toc_title_width_fits_list_content() {
    let mut entries = toc(&[(2, 0), (3, 10), (2, 20)]);
    entries[1].title = "A very long subsection heading that needs a visible ellipsis".to_string();
    let mut app = make_app_with_toc(100, 10, entries);
    app.toggle_toc();
    render_app(&mut app, 80, 12);

    let content_width = app.toc_list_area.unwrap().width.saturating_sub(2) as usize;
    let secondary = &app.visible_toc_lines(8)[1];

    assert!(
        secondary.width() <= content_width,
        "secondary TOC line width {} should fit content width {}",
        secondary.width(),
        content_width
    );
    assert!(line_plain_text(secondary).ends_with('\u{2026}'));
}

#[test]
fn narrow_terminal_toc_split_render_and_sync_do_not_panic() {
    let (ss, _theme) = test_assets();
    let ts = ThemeSet::load_defaults();
    let mut app = make_app_with_toc(100, 5, many_toc(10));
    app.toggle_toc();

    render_app(&mut app, 10, 5);
    assert_eq!(app.toc_width(), DEFAULT_TOC_WIDTH);
    assert_eq!(
        app.effective_toc_width(10),
        App::clamp_toc_width_value(DEFAULT_TOC_WIDTH, 10)
    );
    assert!(app.content_area.x <= 10);
    sync_render_width_for_app(10, &mut app, &ss, &ts);
}
