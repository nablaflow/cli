use crate::aerocloud::{
    Client, fmt_progenitor_err,
    types::{ListPageProjectsV7, PaginationOffset, ProjectStatus, ProjectV7},
    wizard::{STYLE_ACCENT, STYLE_BOLD, STYLE_ERROR},
};
use color_eyre::eyre;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Rect},
    symbols::border,
    text::{Line, Span},
    widgets::{
        Block, Cell, Clear, HighlightSpacing, Paragraph, Row, StatefulWidget,
        Table, TableState, Widget, Wrap,
    },
};
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub enum WidgetResult {
    Exit,
    Selected(ProjectV7),
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Loading,
    Failed(String),
    Selecting {
        projects: Vec<ProjectV7>,
        table_state: TableState,
    },
}

#[derive(Debug, Clone)]
pub struct ProjectPicker {
    client: Client,
    state: Arc<RwLock<State>>,
}

impl ProjectPicker {
    pub fn new(client: Client) -> Self {
        let this = Self {
            client,
            state: Default::default(),
        };
        this.refresh_projects();
        this
    }

    fn refresh_projects(&self) {
        let this = self.clone();

        tokio::spawn(async move {
            if let Err(err) = this.fetch_projects().await {
                *this.state.write().expect("failed to get write lock") =
                    State::Failed(err.to_string());
            }
        });
    }

    async fn fetch_projects(&self) -> eyre::Result<()> {
        {
            *self.state.write().expect("failed to get write lock") =
                State::Loading;
        }

        let mut projects = vec![];
        let mut offset = PaginationOffset(0u64);

        loop {
            let ListPageProjectsV7 { items, nav } = self
                .client
                .projects_v7_list(
                    None,
                    Some(&offset),
                    Some(ProjectStatus::Active),
                )
                .await
                .map_err(fmt_progenitor_err)?
                .into_inner();

            projects.extend(items);

            if let Some(next_offset) = nav.next_offset {
                offset = PaginationOffset(next_offset);
            } else {
                break;
            }
        }

        {
            let mut table_state = TableState::default();
            table_state.select_first();

            *self.state.write().expect("failed to get write lock") =
                State::Selecting {
                    projects,
                    table_state,
                };
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<WidgetResult> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match *self.state.write().expect("failed to get write lock") {
                    State::Loading => {
                        if key_event.code == KeyCode::Esc {
                            return Some(WidgetResult::Exit);
                        }
                    }
                    State::Failed(..) => match key_event.code {
                        KeyCode::Char('r') => {
                            self.refresh_projects();
                        }
                        KeyCode::Esc => {
                            return Some(WidgetResult::Exit);
                        }
                        _ => {}
                    },
                    State::Selecting {
                        ref mut projects,
                        ref mut table_state,
                    } => match key_event.code {
                        KeyCode::Up => {
                            table_state.select_previous();
                        }
                        KeyCode::Down => {
                            table_state.select_next();
                        }
                        KeyCode::Char('r') => {
                            self.refresh_projects();
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = table_state.selected() {
                                return Some(WidgetResult::Selected(
                                    projects.remove(idx),
                                ));
                            }
                        }
                        KeyCode::Esc => {
                            return Some(WidgetResult::Exit);
                        }
                        _ => {}
                    },
                }
            }
            _ => {}
        }

        None
    }
}

impl Widget for &ProjectPicker {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let title =
            Line::from(Span::style(" Select a project ".into(), STYLE_BOLD));

        let instructions = Line::from(vec![
            " (".into(),
            Span::style("esc".into(), STYLE_ACCENT),
            ") quit | (".into(),
            Span::style("↑".into(), STYLE_ACCENT),
            ") move up | (".into(),
            Span::style("↓".into(), STYLE_ACCENT),
            ") move down | (".into(),
            Span::style("enter".into(), STYLE_ACCENT),
            ") select | (".into(),
            Span::style("r".into(), STYLE_ACCENT),
            ") refresh ".into(),
        ]);

        let main_block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        match *self.state.write().expect("failed to get write lock") {
            State::Loading => {
                let inner_area = main_block
                    .inner(area)
                    .centered_vertically(Constraint::Percentage(20));

                Line::from(Span::styled("Loading...", STYLE_BOLD))
                    .centered()
                    .render(inner_area, buf);

                main_block.render(area, buf);
            }
            State::Failed(ref err) => {
                let inner_area = main_block
                    .inner(area)
                    .centered_vertically(Constraint::Percentage(20));

                Paragraph::new(Span::styled(
                    format!("Failed: {err}"),
                    STYLE_ERROR,
                ))
                .wrap(Wrap { trim: true })
                .centered()
                .render(inner_area, buf);

                main_block.render(area, buf);
            }
            State::Selecting {
                ref projects,
                ref mut table_state,
            } => {
                let widths = [Constraint::Fill(1), Constraint::Length(40)];

                let table = Table::new(projects.iter(), widths)
                    .header(
                        Row::new(vec![
                            Cell::from(Span::styled("Name", STYLE_BOLD)),
                            Cell::from(Span::styled("Created at", STYLE_BOLD)),
                        ])
                        .top_margin(1)
                        .bottom_margin(1),
                    )
                    .block(main_block)
                    .highlight_spacing(HighlightSpacing::Always)
                    .highlight_symbol(">> ")
                    .row_highlight_style(STYLE_ACCENT);

                StatefulWidget::render(table, area, buf, table_state);
            }
        }
    }
}

impl From<&ProjectV7> for Row<'_> {
    fn from(p: &ProjectV7) -> Self {
        Row::new(vec![p.name.clone(), format!("{}", p.created_at)])
    }
}
