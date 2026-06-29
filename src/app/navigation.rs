use super::App;

pub(super) enum CycleDirection {
    Forward,
    Backward,
}

pub(super) struct NumkeyCycleState {
    pub(super) key: u8,
    pub(super) position: usize,
}

impl App {
    pub(super) fn enable_toc_active_follow(&mut self) {
        self.toc_follow_active = true;
    }

    pub(crate) fn toc_scroll(&self) -> usize {
        self.toc_scroll
    }

    pub(crate) fn max_toc_scroll(&self, viewport_height: usize) -> usize {
        if viewport_height == 0 {
            0
        } else {
            self.toc_display_entries
                .len()
                .saturating_sub(viewport_height)
        }
    }

    pub(crate) fn toc_is_overflowing(&self, viewport_height: usize) -> bool {
        viewport_height > 0 && self.toc_display_entries.len() > viewport_height
    }

    pub(crate) fn clamp_toc_scroll(&mut self, viewport_height: usize) {
        self.toc_scroll = self.toc_scroll.min(self.max_toc_scroll(viewport_height));
    }

    pub(crate) fn scroll_toc_down(&mut self, n: usize, viewport_height: usize) {
        if !self.toc_is_overflowing(viewport_height) {
            self.toc_scroll = 0;
            return;
        }
        self.toc_follow_active = false;
        self.toc_scroll = (self.toc_scroll + n).min(self.max_toc_scroll(viewport_height));
    }

    pub(crate) fn scroll_toc_up(&mut self, n: usize, viewport_height: usize) {
        if !self.toc_is_overflowing(viewport_height) {
            self.toc_scroll = 0;
            return;
        }
        self.toc_follow_active = false;
        self.toc_scroll = self.toc_scroll.saturating_sub(n);
    }

    pub(crate) fn scroll_toc_to(&mut self, position: usize, viewport_height: usize) {
        if !self.toc_is_overflowing(viewport_height) {
            self.toc_scroll = 0;
            return;
        }
        self.toc_follow_active = false;
        self.toc_scroll = position.min(self.max_toc_scroll(viewport_height));
    }

    pub(crate) fn scroll_toc_to_row(&mut self, row_offset: usize, viewport_height: usize) {
        if viewport_height <= 1 {
            self.scroll_toc_to(0, viewport_height);
            return;
        }
        let max_scroll = self.max_toc_scroll(viewport_height);
        let scroll_pos = row_offset.min(viewport_height - 1) * max_scroll / (viewport_height - 1);
        self.scroll_toc_to(scroll_pos, viewport_height);
    }

    pub(crate) fn active_toc_display_index(&self) -> Option<usize> {
        let active_idx = self.toc_active_idx?;
        self.toc_display_entries
            .iter()
            .position(|&entry_idx| entry_idx == active_idx)
    }

    pub(crate) fn ensure_active_toc_visible(&mut self, viewport_height: usize) {
        self.clamp_toc_scroll(viewport_height);
        if !self.toc_follow_active || viewport_height == 0 {
            return;
        }
        let Some(active_display_idx) = self.active_toc_display_index() else {
            return;
        };
        if active_display_idx < self.toc_scroll {
            self.toc_scroll = active_display_idx;
        } else {
            let visible_end = self.toc_scroll.saturating_add(viewport_height);
            if active_display_idx >= visible_end {
                self.toc_scroll = active_display_idx
                    .saturating_add(1)
                    .saturating_sub(viewport_height);
            }
        }
        self.clamp_toc_scroll(viewport_height);
    }

    pub(crate) fn max_scroll(&self) -> usize {
        self.total()
            .saturating_sub(self.content_area.height as usize)
    }

    pub(crate) fn scroll_percent(&self) -> u16 {
        let max = self.max_scroll();
        if max == 0 {
            return 100;
        }
        ((self.scroll * 100) / max).min(100) as u16
    }

    pub(super) fn reset_numkey_state(&mut self) {
        self.numkey_cycle = None;
        self.reverse_mode = false;
    }

    pub(crate) fn toggle_reverse_mode(&mut self) {
        self.reverse_mode = !self.reverse_mode;
    }

    pub(crate) fn scroll_down(&mut self, n: usize) {
        self.reset_numkey_state();
        self.enable_toc_active_follow();
        self.scroll = (self.scroll + n).min(self.max_scroll());
    }

    pub(crate) fn scroll_up(&mut self, n: usize) {
        self.reset_numkey_state();
        self.enable_toc_active_follow();
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub(crate) fn scroll_top(&mut self) {
        self.reset_numkey_state();
        self.enable_toc_active_follow();
        self.scroll = 0;
    }

    pub(crate) fn scroll_bottom(&mut self) {
        self.reset_numkey_state();
        self.enable_toc_active_follow();
        self.scroll = self.max_scroll();
    }

    pub(crate) fn scroll_to(&mut self, position: usize) {
        self.reset_numkey_state();
        self.enable_toc_active_follow();
        self.scroll = position.min(self.max_scroll());
    }

    pub(crate) fn toggle_toc(&mut self) {
        self.toc_visible = !self.toc_visible;
        if !self.toc_visible {
            self.hovered_toc_idx = None;
            self.toc_scrollbar_dragging = false;
        } else {
            self.enable_toc_active_follow();
        }
    }

    pub(crate) fn toggle_line_numbers(&mut self) {
        self.line_number_visible = !self.line_number_visible;
    }

    fn toc_group_for_numkey(&self, key: u8) -> Vec<usize> {
        let mut group = Vec::new();
        let mut top_level_index = 0u8;
        let mut collecting = false;

        for (idx, _entry, display_level) in self.visible_toc_entries() {
            if display_level == 1 {
                if collecting {
                    break;
                }
                top_level_index += 1;
                if top_level_index == key {
                    collecting = true;
                    group.push(idx);
                }
            } else if collecting {
                group.push(idx);
            }
        }
        group
    }

    pub(crate) fn scroll_to_toc_display_line(&mut self, display_idx: usize) {
        if let Some(&entry_idx) = self.toc_display_entries.get(display_idx) {
            if let Some(entry) = self.toc.get(entry_idx) {
                self.scroll_to(entry.line);
            }
        }
    }

    pub(crate) fn cycle_numkey(&mut self, key: u8) {
        let group = self.toc_group_for_numkey(key);
        if group.is_empty() {
            return;
        }

        let direction = if self.reverse_mode {
            CycleDirection::Backward
        } else {
            CycleDirection::Forward
        };

        let position = match self.numkey_cycle.as_ref().filter(|s| s.key == key) {
            Some(state) => match direction {
                CycleDirection::Forward => (state.position + 1) % group.len(),
                CycleDirection::Backward => (state.position + group.len() - 1) % group.len(),
            },
            None => {
                self.reverse_mode = false;
                0
            }
        };

        self.numkey_cycle = Some(NumkeyCycleState { key, position });
        self.enable_toc_active_follow();
        self.scroll = self.toc[group[position]].line.min(self.max_scroll());
    }
}
