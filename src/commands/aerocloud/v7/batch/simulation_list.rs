use crate::commands::aerocloud::v7::batch::{
    STYLE_ACCENT, STYLE_DIMMED, STYLE_ERROR, STYLE_NORMAL, STYLE_SUCCESS,
    STYLE_WARNING,
    simulation_params::{SimulationParams, SubmissionState},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    macros::{line, span},
    style::Style,
    symbols::border,
    text::{Line, Span},
    widgets::{
        Block, HighlightSpacing, List, ListState, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget,
    },
};
use std::mem;

const HORIZONTAL_SCROLL_STEP: u16 = 8;
const HIGHLIGHT_SYMBOL: &str = ">> ";

#[derive(Debug)]
pub struct SimulationListState {
    list: ListState,
    left: u16,
}

impl Default for SimulationListState {
    fn default() -> Self {
        Self {
            list: ListState::default().with_selected(Some(0)),
            left: 0,
        }
    }
}

impl SimulationListState {
    pub const fn selected(&self) -> Option<usize> {
        self.list.selected()
    }

    pub fn select_previous(&mut self) {
        self.list.select_previous();
    }

    pub fn select_next(&mut self) {
        self.list.select_next();
    }

    pub const fn pan_left(&mut self) {
        self.left = self.left.saturating_sub(HORIZONTAL_SCROLL_STEP);
    }

    pub const fn pan_right(&mut self) {
        self.left = self.left.saturating_add(HORIZONTAL_SCROLL_STEP);
    }
}

pub struct SimulationList<'a> {
    pub has_focus: bool,
    pub is_dimmed: bool,
    pub sims: &'a [SimulationParams],
}

impl SimulationList<'_> {
    fn block(&self) -> Block<'_> {
        let block = Block::bordered()
            .title(
                line![format!(" Simulations ({}) ", self.sims.len())].centered(),
            )
            .border_set(border::PLAIN)
            .border_style(if self.has_focus {
                STYLE_NORMAL
            } else {
                STYLE_DIMMED
            })
            .style(if self.is_dimmed {
                STYLE_DIMMED
            } else {
                STYLE_NORMAL
            });

        if self.has_focus && !self.sims.is_empty() {
            block.title_bottom(Self::instructions().centered())
        } else {
            block
        }
    }

    fn instructions() -> Line<'static> {
        line![
            " (",
            span!(STYLE_ACCENT; "↑/↓"),
            ") select | (",
            span!(STYLE_ACCENT; "←/→"),
            ") pan ",
        ]
    }

    fn line(p: &SimulationParams) -> Line<'_> {
        let style = if p.selected {
            STYLE_NORMAL
        } else {
            STYLE_DIMMED
        };

        let mut spans = vec![span!(p.params.name.as_str()), span!(" ")];

        match p.submission_state {
            SubmissionState::Ready => {}
            SubmissionState::Sending => {
                spans.push(span!(STYLE_WARNING; "(sending...) "));
            }
            SubmissionState::Error(..) => {
                spans.push(span!(STYLE_ERROR; "(error) "));
            }
            SubmissionState::Sent { .. } => {
                spans.push(span!(STYLE_SUCCESS; "(sent) "));
            }
        }

        if p.model_params.is_empty() {
            spans.push(span!(STYLE_ERROR; "(no files) "));
        }

        Line::from(spans).style(style)
    }
}

impl StatefulWidget for &SimulationList<'_> {
    type State = SimulationListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = self.block();

        let lines: Vec<Line<'_>> =
            self.sims.iter().map(SimulationList::line).collect();

        // NOTE: stop panning once the longest line is fully visible, the highlight symbol
        // is always shown and does not pan.
        let max_width = lines.iter().map(Line::width).max().unwrap_or(0);
        let viewport_width = block
            .inner(area)
            .width
            .saturating_sub(u16::try_from(HIGHLIGHT_SYMBOL.len()).unwrap_or(0));
        let max_left = u16::try_from(max_width)
            .unwrap_or(u16::MAX)
            .saturating_sub(viewport_width);
        state.left = state.left.min(max_left);

        let list = List::new(
            lines
                .into_iter()
                .map(|line| skip_columns(line, usize::from(state.left))),
        )
        .block(block)
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol(HIGHLIGHT_SYMBOL)
        .highlight_style(STYLE_ACCENT);

        StatefulWidget::render(list, area, buf, &mut state.list);

        // NOTE: read the selection after rendering, as `List` clamps it to the items count.
        let mut scrollbar_state = ScrollbarState::new(self.sims.len())
            .position(state.list.selected().unwrap_or(0));

        StatefulWidget::render(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            buf,
            &mut scrollbar_state,
        );
    }
}

/// Drops the first `n` columns of `line`, keeping styles. A wide grapheme cut in half is
/// replaced by spaces so that the remaining content stays aligned.
fn skip_columns(mut line: Line<'_>, mut n: usize) -> Line<'_> {
    if n == 0 {
        return line;
    }

    let mut spans = Vec::with_capacity(line.spans.len());

    for span in mem::take(&mut line.spans) {
        if n == 0 {
            spans.push(span);
            continue;
        }

        let mut content = String::new();

        for grapheme in span.styled_graphemes(Style::default()) {
            let width = Span::raw(grapheme.symbol).width();

            if n == 0 {
                content.push_str(grapheme.symbol);
            } else if width <= n {
                n -= width;
            } else {
                content.push_str(&" ".repeat(width - n));
                n = 0;
            }
        }

        spans.push(Span::styled(content, span.style));
    }

    line.spans = spans;
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn skip_columns_across_spans() {
        let line = Line::from(vec![Span::raw("abc"), Span::raw("def")]);

        assert_eq!(text(&skip_columns(line.clone(), 0)), "abcdef");
        assert_eq!(text(&skip_columns(line.clone(), 2)), "cdef");
        assert_eq!(text(&skip_columns(line.clone(), 4)), "ef");
        assert_eq!(text(&skip_columns(line, 10)), "");
    }

    #[test]
    fn skip_columns_splits_wide_graphemes() {
        let line = Line::from("日本");

        assert_eq!(text(&skip_columns(line.clone(), 1)), " 本");
        assert_eq!(text(&skip_columns(line, 2)), "本");
    }
}
