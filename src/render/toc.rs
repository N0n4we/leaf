use crate::{app::App, theme::app_theme};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use super::{CONTENT_HORIZONTAL_PADDING, SCROLLBAR_WIDTH};

pub(super) fn render_toc_panel(f: &mut Frame, app: &mut App, area: Rect, header_area: Rect) {
    let theme = app_theme();
    app.set_toc_panel_width(area.width);
    app.refresh_toc_cache();
    let header_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(header_area);
    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    app.toc_list_area = Some(list_chunks[1]);
    let list_height = list_chunks[1].height as usize;
    app.ensure_active_toc_visible(list_height);

    f.render_widget(
        Paragraph::new("")
            .style(Style::default().bg(theme.ui.toc_bg))
            .block(
                Block::default()
                    .borders(Borders::RIGHT | Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.ui.toc_border))
                    .style(Style::default().bg(theme.ui.toc_bg)),
            ),
        header_chunks[0],
    );

    let toc_scroll = app.toc_scroll();
    let mut lines: Vec<Line<'static>> = app.visible_toc_lines(list_height).to_vec();
    if let Some(display_idx) = app.hovered_toc_idx {
        let is_active = app.toc_display_entries().get(display_idx).copied() == app.toc_active_idx;
        if !is_active {
            if let Some(visible_idx) = display_idx.checked_sub(toc_scroll) {
                if let Some(line) = lines.get_mut(visible_idx) {
                    apply_toc_hover_style(line, theme.ui.toc_hover_fg);
                }
            }
        }
    }
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.ui.toc_bg)),
        list_chunks[1],
    );
    render_toc_scrollbar(f, app, list_chunks[1], list_height);
    f.render_widget(
        Paragraph::new(vec![app.toc_header_line().clone()])
            .style(Style::default().bg(theme.ui.toc_bg)),
        Rect {
            x: header_chunks[0].x,
            y: header_chunks[0].y.saturating_add(1),
            width: header_chunks[0].width.saturating_sub(1),
            height: 1,
        },
    );
}

fn render_toc_scrollbar(f: &mut Frame, app: &App, area: Rect, list_height: usize) {
    if !app.toc_is_overflowing(list_height) {
        return;
    }

    let theme = app_theme();
    let max_scroll = app.max_toc_scroll(list_height);
    let (mouse_col, mouse_row) = app.mouse_position;
    let on_scrollbar = is_on_toc_scrollbar(area, mouse_col, mouse_row);
    let track_len = area.height as usize;
    let mouse_on_thumb = on_scrollbar && track_len > 0 && max_scroll > 0 && {
        let thumb_size = (track_len * track_len / max_scroll).max(1).min(track_len);
        let max_offset = track_len.saturating_sub(thumb_size);
        let thumb_offset = app.toc_scroll() * max_offset / max_scroll;
        let thumb_top = area.y as usize + thumb_offset;
        let thumb_bottom = thumb_top + thumb_size;
        let row = mouse_row as usize;
        row >= thumb_top && row < thumb_bottom
    };

    let mut scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some("│"))
        .thumb_symbol("█");
    if mouse_on_thumb || app.toc_scrollbar_dragging {
        scrollbar = scrollbar.thumb_style(Style::default().fg(theme.ui.scrollbar_hover));
    }

    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(app.toc_scroll());
    f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

fn is_on_toc_scrollbar(area: Rect, col: u16, row: u16) -> bool {
    area.width > 0 && {
        let sb_x = area.x + area.width - 1;
        col == sb_x && row >= area.y && row < area.y + area.height
    }
}

fn apply_toc_hover_style(line: &mut Line<'static>, hover_fg: Color) {
    for span in &mut line.spans {
        span.style = span.style.fg(hover_fg);
    }
}

pub(crate) fn toc_header_line() -> Line<'static> {
    let theme = app_theme();
    Line::from(vec![Span::styled(
        "  TABLE OF CONTENTS",
        Style::default()
            .fg(theme.ui.toc_header_fg)
            .bg(theme.ui.toc_bg)
            .add_modifier(Modifier::BOLD),
    )])
}

pub(crate) fn build_toc_line_with_index(
    entry: &crate::markdown::toc::TocEntry,
    display_level: u8,
    top_level_index: Option<usize>,
    active: bool,
    panel_width: u16,
) -> Line<'static> {
    let theme = app_theme();
    let active_bg = theme.ui.toc_active_bg;
    let inactive_bg = theme.ui.toc_inactive_bg;
    let title_width = toc_title_width(panel_width, display_level);

    match display_level {
        1 => {
            let index = top_level_index.unwrap_or(0) + 1;
            let title = crate::markdown::truncate_display_width(&entry.title, title_width);
            let bg = if active { active_bg } else { inactive_bg };
            Line::from(vec![
                Span::styled(
                    if active { "▎" } else { " " },
                    Style::default().fg(theme.ui.toc_accent).bg(bg),
                ),
                Span::styled("  ", Style::default().bg(bg)),
                Span::styled(
                    format!("{index:02}"),
                    Style::default()
                        .fg(if active {
                            theme.ui.toc_accent
                        } else {
                            theme.ui.toc_index_inactive
                        })
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ", Style::default().bg(bg)),
                Span::styled(
                    title,
                    Style::default()
                        .fg(if active {
                            theme.ui.toc_primary_active
                        } else {
                            theme.ui.toc_primary_inactive
                        })
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        }
        _ => Line::from(vec![
            Span::styled(
                if active { "▎" } else { " " },
                Style::default().fg(theme.ui.toc_accent),
            ),
            Span::raw("     "),
            Span::styled(
                "•",
                Style::default().fg(if active {
                    theme.ui.toc_accent
                } else {
                    theme.ui.toc_secondary_inactive
                }),
            ),
            Span::raw(" "),
            Span::styled(
                crate::markdown::truncate_display_width(&entry.title, title_width),
                Style::default()
                    .fg(if active {
                        theme.ui.toc_secondary_text_active
                    } else {
                        theme.ui.toc_secondary_text_inactive
                    })
                    .add_modifier(if active {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
        ]),
    }
}

const TOC_RIGHT_GUTTER_WIDTH: u16 = CONTENT_HORIZONTAL_PADDING + SCROLLBAR_WIDTH;
const TOC_TOP_LEVEL_PREFIX_WIDTH: u16 = 6;
const TOC_SECONDARY_PREFIX_WIDTH: u16 = 8;

fn toc_title_width(panel_width: u16, display_level: u8) -> usize {
    let prefix_width = if display_level == 1 {
        TOC_TOP_LEVEL_PREFIX_WIDTH
    } else {
        TOC_SECONDARY_PREFIX_WIDTH
    };
    panel_width
        .saturating_sub(TOC_RIGHT_GUTTER_WIDTH)
        .saturating_sub(prefix_width)
        .max(1) as usize
}
