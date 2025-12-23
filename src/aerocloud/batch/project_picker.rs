use crate::aerocloud::{
    Client,
    batch::{Event, STYLE_ACCENT, STYLE_BOLD, STYLE_ERROR},
    fmt_progenitor_err,
    types::{ListPageProjectsV7, PaginationOffset, ProjectStatus, ProjectV7},
};
use color_eyre::eyre;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    symbols::border,
    text::{Line, Span},
    widgets::{
        Block, Cell, Clear, HighlightSpacing, Paragraph, Row, StatefulWidget,
        Table, TableState, Widget, Wrap,
    },
};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct ProjectPicker;

#[derive(Debug, Default)]
pub enum ProjectPickerState {
    #[default]
    Loading,
    Failed(eyre::Report),
    Selecting {
        projects: Vec<ProjectV7>,
        table_state: TableState,
    },
}

pub fn refresh_projects_in_background(client: Client, tx: mpsc::Sender<Event>) {
    tokio::spawn(async move {
        if let Err(err) = fetch_projects(client, tx.clone()).await {
            tx.send(Event::ProjectsLoadingFailed(err))
                .await
                .expect("failed to send");
        }
    });
}

async fn fetch_projects(
    client: Client,
    tx: mpsc::Sender<Event>,
) -> eyre::Result<()> {
    tx.send(Event::ProjectsLoading).await?;

    let mut projects = vec![];
    let mut offset = PaginationOffset(0u64);

    loop {
        let ListPageProjectsV7 { items, nav } = client
            .projects_v7_list(None, Some(&offset), Some(ProjectStatus::Active))
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

    tx.send(Event::ProjectsUpdated(projects)).await?;

    Ok(())
}

impl ProjectPickerState {
    pub async fn handle_event(
        &mut self,
        event: Event,
        client: Client,
        tx: mpsc::Sender<Event>,
    ) -> eyre::Result<()> {
        if let Event::KeyPressed(key_event) = &event
            && (key_event.code == KeyCode::Esc
                || (key_event.code == KeyCode::Char('c')
                    && key_event.modifiers == KeyModifiers::CONTROL))
        {
            tx.send(Event::Exit).await?;

            return Ok(());
        }

        if let Event::ProjectsUpdated(projects) = event {
            let mut table_state = TableState::default();
            table_state.select_first();

            *self = Self::Selecting {
                projects,
                table_state,
            };

            return Ok(());
        }

        if let Event::ProjectsLoading = event {
            *self = Self::Loading;

            return Ok(());
        }

        match self {
            Self::Loading => {
                if let Event::ProjectsLoadingFailed(err) = event {
                    *self = Self::Failed(err);
                }
            }
            Self::Failed(..) => match event {
                Event::KeyPressed(key_event)
                    if key_event.code == KeyCode::Char('r') =>
                {
                    refresh_projects_in_background(client, tx);
                }
                _ => {}
            },
            Self::Selecting {
                projects,
                table_state,
            } => match event {
                Event::KeyPressed(key_event) if key_event.code == KeyCode::Up => {
                    table_state.select_previous();
                }
                Event::KeyPressed(key_event)
                    if key_event.code == KeyCode::Down =>
                {
                    table_state.select_next();
                }
                Event::KeyPressed(key_event)
                    if key_event.code == KeyCode::Char('r') =>
                {
                    refresh_projects_in_background(client, tx);
                }
                Event::KeyPressed(key_event)
                    if key_event.code == KeyCode::Enter =>
                {
                    if let Some(idx) = table_state.selected() {
                        tx.send(Event::ProjectSelected(projects.remove(idx)))
                            .await?;
                    }
                }
                _ => {}
            },
        }

        Ok(())
    }
}

impl StatefulWidget for &ProjectPicker {
    type State = ProjectPickerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
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

        match state {
            ProjectPickerState::Loading => {
                let inner_area = main_block
                    .inner(area)
                    .centered_vertically(Constraint::Percentage(20));

                Line::from(Span::styled("Loading...", STYLE_BOLD))
                    .centered()
                    .render(inner_area, buf);

                main_block.render(area, buf);
            }
            ProjectPickerState::Failed(err) => {
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
            ProjectPickerState::Selecting {
                projects,
                table_state,
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
