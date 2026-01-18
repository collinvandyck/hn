//! Custom list widget for comments with partial item rendering.
//!
//! Unlike Ratatui's standard List widget which skips items that don't fit
//! entirely, this widget renders partial items at viewport boundaries,
//! filling the available space without gaps.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, StatefulWidget, Widget},
};

/// State for the CommentList widget.
#[derive(Default)]
pub struct CommentListState {
    selected: Option<usize>,
}

impl CommentListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }
}

/// A single item in the comment list, containing multiple lines.
pub struct CommentListItem<'a> {
    lines: Vec<Line<'a>>,
}

impl<'a> CommentListItem<'a> {
    pub fn new(lines: Vec<Line<'a>>) -> Self {
        Self { lines }
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }
}

/// A list widget that renders partial items at viewport boundaries.
pub struct CommentList<'a> {
    items: Vec<CommentListItem<'a>>,
    block: Option<Block<'a>>,
    highlight_style: Style,
    highlight_symbol: &'a str,
}

impl<'a> CommentList<'a> {
    pub fn new(items: Vec<CommentListItem<'a>>) -> Self {
        Self {
            items,
            block: None,
            highlight_style: Style::default(),
            highlight_symbol: "",
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = symbol;
        self
    }
}

impl<'a> StatefulWidget for CommentList<'a> {
    type State = CommentListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = match &self.block {
            Some(block) => {
                let inner = block.inner(area);
                block.clone().render(area, buf);
                inner
            }
            None => area,
        };

        if inner.width == 0 || inner.height == 0 || self.items.is_empty() {
            return;
        }

        let item_heights: Vec<usize> = self.items.iter().map(|i| i.height()).collect();
        let viewport_height = inner.height as usize;
        let line_offset = state
            .selected
            .map(|s| calculate_centering_offset(s, &item_heights, viewport_height))
            .unwrap_or(0);

        let symbol_width = self.highlight_symbol.chars().count() as u16;
        let mut current_line = 0;
        let mut y = inner.top();
        let mut selected_first_line_y: Option<u16> = None;

        for (item_idx, item) in self.items.iter().enumerate() {
            let is_selected = state.selected == Some(item_idx);
            let mut is_first_line_of_item = true;

            for line in &item.lines {
                if current_line < line_offset {
                    current_line += 1;
                    is_first_line_of_item = false;
                    continue;
                }
                if y >= inner.bottom() {
                    return;
                }
                // Track the first visible line of the selected item for symbol rendering
                if is_selected && selected_first_line_y.is_none() {
                    selected_first_line_y = Some(y);
                }
                // Apply highlight style for selected item's lines
                if is_selected {
                    buf.set_style(
                        Rect {
                            x: inner.left(),
                            y,
                            width: inner.width,
                            height: 1,
                        },
                        self.highlight_style,
                    );
                }
                // Render highlight symbol on first line of selected item
                if is_selected && is_first_line_of_item {
                    buf.set_string(inner.left(), y, self.highlight_symbol, Style::default());
                }
                let content_x = inner.left() + symbol_width;
                let content_width = inner.width.saturating_sub(symbol_width);
                buf.set_line(content_x, y, line, content_width);
                y += 1;
                current_line += 1;
                is_first_line_of_item = false;
            }
        }
    }
}

fn calculate_centering_offset(
    selected: usize,
    item_heights: &[usize],
    viewport_height: usize,
) -> usize {
    let mut cumulative = vec![0usize];
    for &h in item_heights {
        cumulative.push(cumulative.last().unwrap() + h);
    }
    let total_lines = *cumulative.last().unwrap();
    if total_lines <= viewport_height {
        return 0;
    }
    let selected_start = cumulative.get(selected).copied().unwrap_or(0);
    let selected_height = item_heights.get(selected).copied().unwrap_or(0);
    let selected_center = selected_start + selected_height / 2;
    let half_viewport = viewport_height / 2;
    let ideal_offset = selected_center.saturating_sub(half_viewport);
    let max_offset = total_lines.saturating_sub(viewport_height);
    ideal_offset.min(max_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centering_offset_all_fits() {
        let heights = vec![3, 3, 3];
        let offset = calculate_centering_offset(1, &heights, 20);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_centering_offset_needs_scroll() {
        let heights = vec![5, 5, 5, 5, 5];
        // Total 25 lines, viewport 10
        // Selected item 2 starts at line 10, center at 12
        // Ideal offset = 12 - 5 = 7
        let offset = calculate_centering_offset(2, &heights, 10);
        assert_eq!(offset, 7);
    }

    #[test]
    fn test_centering_offset_clamps_to_max() {
        let heights = vec![5, 5, 5];
        // Total 15 lines, viewport 10, max_offset = 5
        // Selected item 2 starts at 10, center at 12
        // Ideal = 12 - 5 = 7, clamped to 5
        let offset = calculate_centering_offset(2, &heights, 10);
        assert_eq!(offset, 5);
    }

    #[test]
    fn test_centering_offset_first_item() {
        let heights = vec![5, 5, 5];
        // Selected item 0, center at 2
        // Ideal = 2 - 5 = 0 (saturating_sub)
        let offset = calculate_centering_offset(0, &heights, 10);
        assert_eq!(offset, 0);
    }
}
