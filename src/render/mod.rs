mod content;
mod popup;
mod popup_picker;
mod status;
mod toc;

use crate::{
    app::{App, TOC_DIVIDER_WIDTH},
    theme::app_theme,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

#[cfg(test)]
pub(crate) use popup::wrap_path_lines;
pub(crate) use status::build_status_bar;
pub(crate) use toc::{build_toc_line_with_index, toc_header_line};

pub(crate) const CONTENT_HORIZONTAL_PADDING: u16 = 1;
pub(crate) const SCROLLBAR_WIDTH: u16 = 1;

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let (toc_areas, divider_area, content_area): (Option<(Rect, Rect)>, Option<Rect>, Rect) =
        if app.is_toc_visible() && app.has_toc() {
            let split_width = app.effective_toc_width(root[0].width);
            let divider_width = TOC_DIVIDER_WIDTH.min(split_width);
            let toc_panel_width = app.toc_panel_width(root[0].width);

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(toc_panel_width),
                    Constraint::Length(divider_width),
                    Constraint::Min(0),
                ])
                .split(root[0]);

            app.set_toc_split_area(Some(root[0]));
            let divider = (divider_width > 0).then_some(cols[1]);
            app.set_toc_resizer_area(divider);
            let header_area = Rect {
                x: cols[0].x,
                y: cols[0].y,
                width: toc_panel_width.saturating_add(divider_width),
                height: cols[0].height,
            };

            (
                (toc_panel_width > 0).then_some((cols[0], header_area)),
                divider,
                cols[2],
            )
        } else {
            app.clear_toc_layout_state();
            (None, None, root[0])
        };

    if let Some((ta, header_area)) = toc_areas {
        toc::render_toc_panel(f, app, ta, header_area);
    } else if app.is_toc_visible() && app.has_toc() {
        app.toc_list_area = None;
        app.hovered_toc_idx = None;
        app.toc_scrollbar_dragging = false;
    }

    if let Some(area) = divider_area {
        render_toc_divider(f, app, area);
    }

    app.content_area = content_area;
    content::render_content_panel(f, app, content_area);
    content::render_status_bar(f, app, root[1]);

    if app.is_help_open() {
        popup::render_help_popup(f, app);
    } else if app.is_picker_loading() || app.is_picker_load_failed() {
        popup_picker::render_picker_loading_popup(f, app);
    } else if app.is_file_picker_open() {
        popup_picker::render_file_popup(f, app);
    } else if app.is_theme_picker_open() {
        popup::render_theme_popup(f, app);
    } else if app.is_editor_picker_open() {
        popup_picker::render_editor_popup(f, app);
    } else if app.is_path_popup_open() {
        popup::render_path_popup(f, app);
    }
}

fn render_toc_divider(f: &mut Frame, app: &App, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let theme = app_theme();
    let (mouse_col, mouse_row) = app.mouse_position;
    let hovered = mouse_col >= area.x
        && mouse_col < area.x + area.width
        && mouse_row >= area.y
        && mouse_row < area.y + area.height;
    let fg = if hovered || app.toc_resizer_dragging() {
        theme.ui.scrollbar_hover
    } else {
        theme.ui.toc_border
    };
    let style = Style::default().fg(fg).bg(theme.ui.toc_bg);
    let lines = (0..area.height)
        .map(|_| Line::from(Span::styled("│", style)))
        .collect::<Vec<_>>();
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.ui.toc_bg)),
        area,
    );
}

pub(super) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let popup_width = width.min(area.width.saturating_sub(2)).max(1);
    let popup_height = height.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + area.width.saturating_sub(popup_width) / 2,
        y: area.y + area.height.saturating_sub(popup_height) / 2,
        width: popup_width,
        height: popup_height,
    }
}
