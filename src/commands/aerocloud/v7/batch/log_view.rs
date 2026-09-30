use crate::{
    commands::aerocloud::v7::batch::{
        STYLE_ACCENT, STYLE_BOLD, STYLE_DIMMED, STYLE_ERROR, STYLE_NORMAL,
        STYLE_SUCCESS, STYLE_WARNING,
    },
    tracing::LogBuffer,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    macros::{line, span},
    style::Style,
    symbols::border,
    text::Line,
    widgets::{
        Block, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget,
    },
};

const HORIZONTAL_SCROLL_STEP: u16 = 8;

#[derive(Debug, Default, Clone)]
pub struct LogViewState {
    /// Absolute index of the first visible line, `None` when following new lines.
    top: Option<usize>,
    left: u16,
    viewport_height: usize,
}

impl LogViewState {
    /// Returns whether the view should stay open.
    pub fn handle_key(&mut self, key_event: KeyEvent, logs: &LogBuffer) -> bool {
        let (first, last_top) = self.bounds(logs);
        let curr = self.top.unwrap_or(last_top).clamp(first, last_top);
        let page = self.viewport_height.max(1);

        let next = match key_event.code {
            KeyCode::Esc | KeyCode::Char('q' | 'l') => return false,
            KeyCode::Up | KeyCode::Char('k') => curr.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => curr + 1,
            KeyCode::PageUp => curr.saturating_sub(page),
            KeyCode::PageDown => curr + page,
            KeyCode::Home | KeyCode::Char('g') => first,
            KeyCode::End | KeyCode::Char('G') => last_top,
            KeyCode::Left => {
                self.left = self.left.saturating_sub(HORIZONTAL_SCROLL_STEP);
                curr
            }
            KeyCode::Right => {
                self.left = self.left.saturating_add(HORIZONTAL_SCROLL_STEP);
                curr
            }
            _ => curr,
        };

        // NOTE: reaching the bottom resumes following.
        self.top = (next < last_top).then(|| next.max(first));

        true
    }

    /// Absolute indices of the first buffered line and of the last possible top line.
    fn bounds(&self, logs: &LogBuffer) -> (usize, usize) {
        logs.with_lines(|lines, appended| {
            let first = appended - lines.len();
            (
                first,
                first + lines.len().saturating_sub(self.viewport_height),
            )
        })
    }
}

pub struct LogView<'a> {
    pub logs: &'a LogBuffer,
}

impl LogView<'_> {
    fn style_for_line(line: &str) -> Style {
        // NOTE: level is right after the timestamp, e.g. `2026-01-01T00:00:00.000000Z  WARN ...`.
        match line.split_whitespace().nth(1) {
            Some("ERROR") => STYLE_ERROR,
            Some("WARN") => STYLE_WARNING,
            Some("DEBUG" | "TRACE") => STYLE_DIMMED,
            _ => STYLE_NORMAL,
        }
    }
}

impl StatefulWidget for &LogView<'_> {
    type State = LogViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let status = if state.top.is_none() {
            span!(STYLE_SUCCESS; "[following] ")
        } else {
            span!(STYLE_DIMMED; "[paused] ")
        };

        let instructions = line![
            " (",
            span!(STYLE_ACCENT; "↑/↓"),
            ") scroll | (",
            span!(STYLE_ACCENT; "pgup/pgdn"),
            ") page | (",
            span!(STYLE_ACCENT; "home/end"),
            ") top/follow | (",
            span!(STYLE_ACCENT; "←/→"),
            ") pan | (",
            span!(STYLE_ACCENT; "l/esc"),
            ") close ",
        ];

        let block = Block::bordered()
            .title(
                line![
                    span!(STYLE_BOLD; " Logs "),
                    status,
                    span!(STYLE_DIMMED; format!("{} ", self.logs.path().display())),
                ]
                .centered(),
            )
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let inner = block.inner(area);
        state.viewport_height = usize::from(inner.height);

        // NOTE: copy out visible lines so the lock is not held while rendering, which would
        // deadlock if anything logged in the meantime.
        let (offset, len, visible) = self.logs.with_lines(|lines, appended| {
            let first = appended - lines.len();
            let last_top =
                first + lines.len().saturating_sub(state.viewport_height);
            let top = state.top.unwrap_or(last_top).clamp(first, last_top);

            // Keep the paused position valid after old lines were evicted.
            if state.top.is_some() {
                state.top = Some(top);
            }

            let offset = top - first;
            let visible: Vec<String> = lines
                .iter()
                .skip(offset)
                .take(state.viewport_height)
                .cloned()
                .collect();

            (offset, lines.len(), visible)
        });

        let lines: Vec<Line<'_>> = visible
            .into_iter()
            .map(|l| {
                let style = LogView::style_for_line(&l);
                Line::styled(l, style)
            })
            .collect();

        Widget::render(&Clear, area, buf);

        Paragraph::new(lines)
            .scroll((0, state.left))
            .block(block)
            .render(area, buf);

        let mut scrollbar_state =
            ScrollbarState::new(len.saturating_sub(state.viewport_height))
                .viewport_content_length(state.viewport_height)
                .position(offset);

        StatefulWidget::render(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            buf,
            &mut scrollbar_state,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use std::{io::Write, path::PathBuf};

    fn press(state: &mut LogViewState, code: KeyCode, logs: &LogBuffer) -> bool {
        state.handle_key(KeyEvent::new(code, KeyModifiers::NONE), logs)
    }

    #[test]
    fn scrolling_pauses_and_reaching_bottom_follows_again() {
        let logs = LogBuffer::new(PathBuf::new());
        for i in 0..20 {
            writeln!(&logs, "{i}").unwrap();
        }

        let mut state = LogViewState {
            viewport_height: 5,
            ..Default::default()
        };

        assert!(press(&mut state, KeyCode::Up, &logs));
        assert_eq!(state.top, Some(14));

        assert!(press(&mut state, KeyCode::Home, &logs));
        assert_eq!(state.top, Some(0));

        assert!(press(&mut state, KeyCode::PageDown, &logs));
        assert_eq!(state.top, Some(5));

        assert!(press(&mut state, KeyCode::End, &logs));
        assert_eq!(state.top, None);

        assert!(!press(&mut state, KeyCode::Esc, &logs));
    }
}
