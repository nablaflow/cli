use crate::aerocloud::{
    Client,
    types::ProjectV7,
    wizard::{
        project_picker::{
            ProjectPicker, ProjectPickerState, refresh_projects_in_background,
        },
        simulation_detail::SimulationDetail,
        simulation_params::SimulationParams,
    },
};
use color_eyre::eyre::{self, WrapErr};
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers,
};
use futures_util::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect, Size, Spacing},
    style::Style,
    symbols::border,
    text::{Line, Span, Text},
    widgets::{
        Block, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};
use std::{borrow::Cow, path::Path};
use tokio::sync::mpsc;

mod project_picker;
mod simulation_detail;
mod simulation_params;

// Made using https://budavariam.github.io/asciiart-text/multi variant `ANSI Shadow`
const LOGO_ASCII_ART: &str = include_str!("logo.txt");

const STYLE_NORMAL: Style = Style::new();
const STYLE_DIMMED: Style = Style::new().dim();
const STYLE_BOLD: Style = Style::new().bold();
const STYLE_ACCENT: Style = Style::new().blue().bold();
const STYLE_ERROR: Style = Style::new().red().bold();
const STYLE_WARNING: Style = Style::new().yellow().bold();

const MIN_TERM_SIZE: Size = Size::new(100, 38);

pub async fn run(client: &Client, root_dir: Option<&Path>) -> eyre::Result<()> {
    let sims = if let Some(root_dir) = root_dir {
        let sims = SimulationParams::many_from_root_dir(root_dir)?;

        if sims.is_empty() {
            eyre::bail!("no simulations found in `{}`", root_dir.display());
        }

        sims
    } else {
        vec![]
    };

    let mut app = Wizard::new(client.clone(), sims);

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal).await;

    ratatui::restore();

    result
}

#[derive(Debug)]
struct Wizard {
    client: Client,
    running: bool,
    term_size: Size,

    simulations: Vec<SimulationParams>,

    state: State,
}

#[derive(Debug, Clone)]
enum ActiveState {
    ViewingList,
    ViewingDetail,
    ConfirmExit { prev: Box<ActiveState> },
    ConfirmSubmit,
}

impl ActiveState {
    fn toggle_viewing_focus(&mut self) {
        match self {
            Self::ViewingList => *self = Self::ViewingDetail,
            Self::ViewingDetail => *self = Self::ViewingList,
            _ => {}
        }
    }
}

#[derive(Debug)]
enum State {
    Init,
    PickingProject {
        state: ProjectPickerState,
    },
    Active {
        state: ActiveState,
        project: ProjectV7,
        sims_list_state: ListState,
        sim_detail_scrollbar_state: ScrollbarState,
    },
}

#[derive(Debug)]
pub enum Event {
    KeyPressed(crossterm::event::KeyEvent),
    TerminalResized(Size),
    ProjectsLoading,
    ProjectsUpdated(Vec<ProjectV7>),
    ProjectsLoadingFailed(eyre::Report),
    ProjectSelected(ProjectV7),
    Exit,
}

async fn handle_term_events(tx: mpsc::Sender<Event>) -> eyre::Result<()> {
    let mut event_stream = EventStream::default();

    while let Some(Ok(event)) = event_stream.next().await {
        match event {
            CrosstermEvent::Key(key_event)
                if key_event.kind == KeyEventKind::Press =>
            {
                tx.send(Event::KeyPressed(key_event)).await?;
            }
            CrosstermEvent::Resize(w, h) => {
                tx.send(Event::TerminalResized(Size::new(w, h))).await?;
            }
            _ => {}
        }
    }

    Ok(())
}

impl Wizard {
    fn new(client: Client, simulations: Vec<SimulationParams>) -> Self {
        Self {
            state: State::Init,
            running: false,
            term_size: Size::default(),
            simulations,
            client,
        }
    }

    async fn run(&mut self, terminal: &mut DefaultTerminal) -> eyre::Result<()> {
        self.term_size = terminal.size().wrap_err("getting term size")?;
        self.running = true;

        let (event_tx, mut event_rx) = mpsc::channel(10);

        tokio::spawn(handle_term_events(event_tx.clone()));

        if let State::Init = self.state {
            refresh_projects_in_background(self.client.clone(), event_tx.clone());

            self.state = State::PickingProject {
                state: ProjectPickerState::default(),
            };
        }

        while self.running {
            let event = event_rx
                .recv()
                .await
                .ok_or_else(|| eyre::eyre!("polling for events"))?;

            tracing::debug!(?event);

            self.handle_event(event, event_tx.clone()).await?;

            terminal.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    async fn handle_event(
        &mut self,
        event: Event,
        tx: mpsc::Sender<Event>,
    ) -> eyre::Result<()> {
        if let Event::TerminalResized(size) = event {
            self.term_size = size;
            return Ok(());
        }

        if let Event::Exit = event {
            self.immediate_exit();
            return Ok(());
        }

        match self.state {
            State::Init => {}
            State::PickingProject { ref mut state } => {
                if let Event::ProjectSelected(project) = event {
                    self.state = State::Active {
                        project,
                        state: ActiveState::ViewingList,
                        sims_list_state: ListState::default()
                            .with_selected(Some(0)),
                        sim_detail_scrollbar_state: ScrollbarState::default(),
                    };
                } else {
                    state.handle_event(event, self.client.clone(), tx).await?;
                }
            }
            State::Active { .. } => self.handle_event_state_active(&event),
        }

        Ok(())
    }

    fn handle_event_state_active(&mut self, event: &Event) {
        let State::Active {
            ref mut state,
            ref mut sims_list_state,
            ref mut sim_detail_scrollbar_state,
            ..
        } = self.state
        else {
            return;
        };

        if let Event::KeyPressed(key_event) = event {
            match state {
                ActiveState::ViewingList => {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Up, _) => {
                            sims_list_state.select_previous();
                            sim_detail_scrollbar_state.first();
                        }
                        (KeyCode::Down, _) => {
                            sims_list_state.select_next();
                            sim_detail_scrollbar_state.first();
                        }
                        (KeyCode::Esc, _)
                        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            *state = ActiveState::ConfirmExit {
                                prev: Box::new(state.clone()),
                            };
                        }
                        (KeyCode::Tab, _) => {
                            state.toggle_viewing_focus();
                        }
                        (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                            *state = ActiveState::ConfirmSubmit;
                        }
                        _ => {}
                    }
                }
                ActiveState::ViewingDetail => {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Up, KeyModifiers::SHIFT) => {
                            sims_list_state.select_previous();
                            sim_detail_scrollbar_state.first();
                        }
                        (KeyCode::Down, KeyModifiers::SHIFT) => {
                            sims_list_state.select_next();
                            sim_detail_scrollbar_state.first();
                        }
                        (KeyCode::Up, _) => {
                            sim_detail_scrollbar_state.prev();
                        }
                        (KeyCode::Down, _) => {
                            sim_detail_scrollbar_state.next();
                        }
                        (KeyCode::Esc, _)
                        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            *state = ActiveState::ConfirmExit {
                                prev: Box::new(state.clone()),
                            };
                        }
                        (KeyCode::Tab, _) => {
                            state.toggle_viewing_focus();
                        }
                        (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                            *state = ActiveState::ConfirmSubmit;
                        }
                        _ => {}
                    }
                }
                ActiveState::ConfirmExit { prev } => match key_event.code {
                    KeyCode::Char('y') => {
                        self.immediate_exit();
                    }
                    KeyCode::Char('n') => {
                        *state = *prev.clone();
                    }
                    _ => {}
                },
                ActiveState::ConfirmSubmit => match key_event.code {
                    KeyCode::Char('y') => {
                        *state = ActiveState::ViewingList;
                    }
                    KeyCode::Char('n') => {
                        *state = ActiveState::ViewingList;
                    }
                    _ => {}
                },
            }
        }
    }

    fn immediate_exit(&mut self) {
        self.running = false;
    }

    fn render_state_picking_project(
        state: &mut ProjectPickerState,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let [upper, lower] = area.layout(
            &Layout::vertical([Constraint::Min(8), Constraint::Fill(1)])
                .flex(Flex::Center)
                .vertical_margin(5)
                .horizontal_margin(10)
                .spacing(Spacing::Space(2)),
        );

        Text::from(LOGO_ASCII_ART)
            .centered()
            .render(upper.centered_vertically(Constraint::Ratio(1, 2)), buf);

        StatefulWidget::render(&ProjectPicker, lower, buf, state);
    }

    fn render_state_active(
        state: &ActiveState,
        simulations: &[SimulationParams],
        sims_list_state: &mut ListState,
        sim_detail_scrollbar_state: &mut ScrollbarState,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let layout = Layout::horizontal([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .vertical_margin(1)
        .horizontal_margin(2);

        let [left_area, right_area] = area.layout(&layout);

        Self::render_sims_list(
            state,
            simulations,
            sims_list_state,
            left_area,
            buf,
        );

        Self::render_sim_detail(
            state,
            simulations,
            sims_list_state,
            sim_detail_scrollbar_state,
            right_area,
            buf,
        );

        match state {
            ActiveState::ConfirmExit { .. } => {
                Wizard::render_exit_popup(area, buf);
            }
            ActiveState::ConfirmSubmit => {
                Wizard::render_submit_confirmation_popup(area, buf);
            }
            _ => {}
        }
    }

    fn render_sim_detail(
        state: &ActiveState,
        simulations: &[SimulationParams],
        sims_list_state: &ListState,
        sim_detail_scrollbar_state: &mut ScrollbarState,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let detail = SimulationDetail {
            focus: matches!(state, ActiveState::ViewingDetail),
            sim: sims_list_state
                .selected()
                .and_then(|idx| simulations.get(idx)),
        };

        StatefulWidget::render(&detail, area, buf, sim_detail_scrollbar_state);
    }

    fn render_sims_list(
        state: &ActiveState,
        simulations: &[SimulationParams],
        sims_list_state: &mut ListState,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let block = Block::bordered()
            .title(
                Line::from(format!(" Simulations ({}) ", simulations.len()))
                    .centered(),
            )
            .border_set(border::PLAIN)
            .style(if matches!(state, ActiveState::ViewingList) {
                STYLE_NORMAL
            } else {
                STYLE_DIMMED
            });

        let list = List::new(simulations.iter())
            .block(block)
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol(">> ")
            .highlight_style(STYLE_ACCENT);

        StatefulWidget::render(list, area, buf, sims_list_state);
    }

    fn render_exit_popup(area: Rect, buf: &mut Buffer) {
        let area = center(
            area,
            Constraint::Percentage(38),
            Constraint::Length(5), // top and bottom border + content
        );

        let instructions = Line::from(vec![
            Span::raw(" ("),
            Span::styled("y", STYLE_ERROR),
            Span::raw(") yes | ("),
            Span::styled("n", STYLE_ERROR),
            Span::raw(") no "),
        ]);

        let block = Block::bordered()
            .title(
                Line::from(Span::styled(" Confirmation ", STYLE_BOLD)).centered(),
            )
            .title_bottom(instructions.centered())
            .border_set(border::THICK)
            .style(STYLE_ERROR);

        let paragraph = Paragraph::new(vec![
            Line::default(),
            Line::raw("Are you sure you want to exit?").centered(),
            Line::default(),
        ])
        .block(block)
        .wrap(Wrap { trim: false });

        Widget::render(&Clear, area, buf);
        Widget::render(&paragraph, area, buf);
    }

    fn render_submit_confirmation_popup(area: Rect, buf: &mut Buffer) {
        let area = center(
            area,
            Constraint::Percentage(38),
            Constraint::Length(5), // top and bottom border + content
        );

        let instructions = Line::from(vec![
            Span::raw(" ("),
            Span::styled("y", STYLE_ACCENT),
            Span::raw(") yes | ("),
            Span::styled("n", STYLE_ACCENT),
            Span::raw(") no "),
        ]);

        let block = Block::bordered()
            .title(
                Line::from(Span::styled(" Launching batch ", STYLE_BOLD))
                    .centered(),
            )
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let paragraph = Paragraph::new(vec![
            Line::default(),
            Line::raw("Are you sure you want to continue?").centered(),
            Line::default(),
        ])
        .block(block)
        .wrap(Wrap { trim: false });

        Widget::render(&Clear, area, buf);
        Widget::render(&paragraph, area, buf);
    }

    fn render_template(&self, area: Rect, buf: &mut Buffer) {
        let style = if matches!(self.state, State::Active { .. }) {
            STYLE_NORMAL
        } else {
            STYLE_DIMMED
        };

        let title: Cow<'_, str> =
            if let State::Active { ref project, .. } = self.state {
                format!(" AeroCloud v7 (project: `{}`) ", project.name).into()
            } else {
                " AeroCloud v7 ".into()
            };

        let instructions = Line::from(vec![
            " (".into(),
            Span::styled("tab", STYLE_ACCENT),
            ") cycle list<->detail | (".into(),
            Span::styled("ctrl+o", STYLE_ACCENT),
            ") submit batch | (".into(),
            Span::styled("esc", STYLE_ACCENT),
            ") quit ".into(),
        ]);

        let block = Block::bordered()
            .title(
                Line::from(Span::styled(title.into_owned(), STYLE_BOLD))
                    .centered(),
            )
            .title_bottom(instructions.style(style).centered())
            .border_set(border::THICK);

        Widget::render(&block, area, buf);
    }

    fn is_term_size_not_enough(&self) -> bool {
        self.term_size.width < MIN_TERM_SIZE.width
            || self.term_size.height < MIN_TERM_SIZE.height
    }

    fn show_min_term_size_notice(&self, area: Rect, buf: &mut Buffer) {
        let paragraph = Paragraph::new(vec![
            Line::default(),
            Line::raw(format!(
                "Terminal size is too small ({}x{}).",
                self.term_size.width, self.term_size.height,
            ))
            .centered(),
            Line::raw(format!(
                "Need at least {}x{}",
                MIN_TERM_SIZE.width, MIN_TERM_SIZE.height
            ))
            .centered(),
            Line::default(),
        ])
        .block(
            Block::bordered()
                .border_set(border::THICK)
                .style(STYLE_ERROR),
        )
        .wrap(Wrap { trim: false });

        Widget::render(&paragraph, area, buf);
    }
}

impl Widget for &mut Wizard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.is_term_size_not_enough() {
            self.show_min_term_size_notice(area, buf);
            return;
        }

        match self.state {
            State::Init => {}
            State::PickingProject { ref mut state } => {
                Wizard::render_state_picking_project(state, area, buf);
            }
            State::Active {
                ref state,
                ref mut sims_list_state,
                ref mut sim_detail_scrollbar_state,
                ..
            } => {
                Wizard::render_state_active(
                    state,
                    &self.simulations,
                    sims_list_state,
                    sim_detail_scrollbar_state,
                    area,
                    buf,
                );
            }
        }

        self.render_template(area, buf);
    }
}

impl From<&SimulationParams> for ListItem<'_> {
    fn from(p: &SimulationParams) -> Self {
        ListItem::from(Line::from(vec![
            Span::raw(p.params.name.clone()),
            " ".into(),
            Span::styled(format!("({})", p.params.quality), STYLE_ACCENT),
        ]))
    }
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
