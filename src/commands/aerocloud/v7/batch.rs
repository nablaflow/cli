use crate::{
    aerocloud::{
        Client,
        types::{ProjectV7, SimulationV7},
    },
    commands::aerocloud::v7::batch::{
        project_picker::{
            ProjectPicker, ProjectPickerState, refresh_projects_in_background,
        },
        simulation_detail::SimulationDetail,
        simulation_params::{SimulationParams, SubmissionState},
        submit::submit_batch_in_background,
    },
    fmt::human_err_report,
};
use bytesize::ByteSize;
use color_eyre::eyre::{self, WrapErr};
use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers,
};
use futures_util::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect, Size, Spacing},
    prelude::Color,
    style::Style,
    symbols::border,
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, HighlightSpacing, List, ListItem,
        ListState, Padding, Paragraph, ScrollbarState, StatefulWidget, Widget,
        Wrap,
    },
};
use std::{
    borrow::Cow,
    mem,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{sync::mpsc, time};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod project_picker;
mod simulation_detail;
mod simulation_params;
mod submit;

// Made using https://budavariam.github.io/asciiart-text/multi variant `ANSI Shadow`
const LOGO_ASCII_ART: &str = include_str!("../../../aerocloud/logo.txt");

const STYLE_NORMAL: Style = Style::new();
const STYLE_DIMMED: Style = Style::new().dim();
const STYLE_BOLD: Style = Style::new().bold();
const STYLE_ACCENT: Style = Style::new().fg(Color::Rgb(0xff, 0xbc, 0x00)).bold();
const STYLE_SUCCESS: Style = Style::new().green().bold();
const STYLE_ERROR: Style = Style::new().red().bold();
const STYLE_WARNING: Style = Style::new().yellow().bold();

const MIN_TERM_SIZE: Size = Size::new(110, 38);

const SLEEP_FOR_FEEDBACK: Duration = Duration::from_millis(100);

pub async fn run(client: &Client, root_dir: Option<&Path>) -> eyre::Result<()> {
    let sims = if let Some(root_dir) = root_dir {
        let sims = SimulationParams::many_from_root_dir(root_dir).await?;

        if sims.is_empty() {
            tracing::error!("no simulations found in `{}`", root_dir.display());

            return Ok(());
        }

        sims
    } else {
        vec![]
    };

    let mut app =
        Batch::new(client.clone(), root_dir.map(ToOwned::to_owned), sims);

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal).await;

    ratatui::restore();

    result
}

pub fn refresh_sims_in_background(root_dir: &Path, tx: mpsc::Sender<Event>) {
    let root_dir = root_dir.to_owned();

    tokio::spawn(async move {
        // NOTE: sleep so that reloading popup is shown and user has visual feedback on the
        // operation.
        time::sleep(SLEEP_FOR_FEEDBACK).await;

        let sims = SimulationParams::many_from_root_dir(&root_dir).await?;
        tx.send(Event::SimsReloaded(sims)).await?;

        Ok::<(), eyre::Report>(())
    });
}

#[derive(Debug)]
struct Batch {
    client: Client,
    running: bool,
    term_size: Size,

    root_dir: Option<PathBuf>,
    simulations: Vec<SimulationParams>,

    state: State,
}

#[derive(Debug, Clone, Default)]
enum ActiveState {
    #[default]
    ViewingList,
    ViewingDetail,
    ConfirmExit {
        prev: Box<ActiveState>,
    },
    ConfirmSubmit,
    ReloadingSims,
    Submitting {
        cancellation_token: CancellationToken,
        bytes_count: ByteSize,
        bytes_progress: ByteSize,
        sims_count: usize,
        sims_progress: usize,
    },
}

#[derive(Debug)]
enum State {
    Init,
    PickingProject {
        state: ProjectPickerState,
    },
    Active {
        state: ActiveState,
        project: Box<ProjectV7>,
        sims_list_state: ListState,
        sim_detail_scrollbar_state: ScrollbarState,
    },
}

#[derive(Debug)]
pub enum Event {
    KeyPressed(crossterm::event::KeyEvent),
    TerminalResized(Size),
    ProjectsLoading,
    ProjectsUpdated(eyre::Result<Vec<ProjectV7>>),
    ProjectSelected(Box<ProjectV7>),
    FileUploaded(ByteSize),
    SimsReloaded(Vec<SimulationParams>),
    SimSubmitted {
        internal_id: Uuid,
        res: eyre::Result<Box<SimulationV7>, eyre::Report>,
    },
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

impl Batch {
    fn new(
        client: Client,
        root_dir: Option<PathBuf>,
        simulations: Vec<SimulationParams>,
    ) -> Self {
        Self {
            state: State::Init,
            running: false,
            term_size: Size::default(),
            root_dir,
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

            tracing::trace!(?event);

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
            State::Active { .. } => {
                self.handle_event_state_active(event, &tx).await?;
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    async fn handle_event_state_active(
        &mut self,
        event: Event,
        tx: &mpsc::Sender<Event>,
    ) -> eyre::Result<()> {
        let State::Active {
            ref project,
            ref mut state,
            ref mut sims_list_state,
            ref mut sim_detail_scrollbar_state,
            ..
        } = self.state
        else {
            return Ok(());
        };

        let mut curr_state = mem::take(state);
        let mut next_state: Option<ActiveState> = None;

        match (&mut curr_state, event) {
            (ActiveState::ViewingList, Event::KeyPressed(key_event)) => {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char(' '), _) => {
                        if let Some(idx) = sims_list_state.selected()
                            && let Some(sim) = self.simulations.get_mut(idx)
                        {
                            sim.selected = !sim.selected;
                        }
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                        if let Some(idx) = sims_list_state.selected()
                            && let Some(sim) = self.simulations.get_mut(idx)
                            && let Err(err) = sim.reset_submission_state().await
                        {
                            tracing::error!(
                                "failed to flush submission state for sim in dir `{}`: {err:?}",
                                sim.dir.display()
                            );
                        }
                    }
                    (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                        if self
                            .simulations
                            .iter()
                            .any(SimulationParams::is_submittable)
                        {
                            next_state = Some(ActiveState::ConfirmSubmit);
                        }
                    }
                    (KeyCode::Char('r'), _) => {
                        if let Some(root_dir) = self.root_dir.as_ref() {
                            refresh_sims_in_background(root_dir, tx.clone());
                        }

                        next_state = Some(ActiveState::ReloadingSims);
                    }
                    (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        next_state = Some(ActiveState::ConfirmExit {
                            prev: Box::new(ActiveState::ViewingList),
                        });
                    }
                    (KeyCode::Up, _) => {
                        sims_list_state.select_previous();
                        sim_detail_scrollbar_state.first();
                    }
                    (KeyCode::Down, _) => {
                        sims_list_state.select_next();
                        sim_detail_scrollbar_state.first();
                    }
                    (KeyCode::Tab, _) => {
                        next_state = Some(ActiveState::ViewingDetail);
                    }
                    _ => {}
                }
            }
            (ActiveState::ViewingDetail, Event::KeyPressed(key_event)) => {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char(' '), _) => {
                        if let Some(idx) = sims_list_state.selected()
                            && let Some(sim) = self.simulations.get_mut(idx)
                        {
                            sim.selected = !sim.selected;
                        }
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                        if let Some(idx) = sims_list_state.selected()
                            && let Some(sim) = self.simulations.get_mut(idx)
                            && let Err(err) = sim.reset_submission_state().await
                        {
                            tracing::error!(
                                "failed to flush submission state for sim in dir `{}`: {err:?}",
                                sim.dir.display()
                            );
                        }
                    }
                    (KeyCode::Char('r'), _) => {
                        if let Some(root_dir) = self.root_dir.as_ref() {
                            refresh_sims_in_background(root_dir, tx.clone());
                        }

                        next_state = Some(ActiveState::ReloadingSims);
                    }
                    (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                        if self
                            .simulations
                            .iter()
                            .any(SimulationParams::is_submittable)
                        {
                            next_state = Some(ActiveState::ConfirmSubmit);
                        }
                    }
                    (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        next_state = Some(ActiveState::ConfirmExit {
                            prev: Box::new(ActiveState::ViewingDetail),
                        });
                    }
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
                    (KeyCode::Tab, _) => {
                        next_state = Some(ActiveState::ViewingList);
                    }
                    _ => {}
                }
            }
            (ActiveState::ConfirmExit { prev }, Event::KeyPressed(key_event)) => {
                match key_event.code {
                    KeyCode::Char('y') => {
                        self.immediate_exit();

                        return Ok(());
                    }
                    KeyCode::Char('n') => {
                        next_state = Some(*prev.clone());
                    }
                    _ => {}
                }
            }
            (ActiveState::ConfirmSubmit, Event::KeyPressed(key_event)) => {
                match key_event.code {
                    KeyCode::Char('y') => {
                        let sims_to_submit: Vec<SimulationParams> = self
                            .simulations
                            .iter()
                            .filter(|sim_params| sim_params.is_submittable())
                            .cloned()
                            .collect();

                        let bytes_count = sims_to_submit
                            .iter()
                            .fold(ByteSize::default(), |acc, sim_params| {
                                acc + sim_params.files_size()
                            });

                        let sims_count = sims_to_submit.len();

                        let cancellation_token = CancellationToken::new();

                        submit_batch_in_background(
                            &project.id,
                            sims_to_submit,
                            &self.client,
                            &cancellation_token,
                            tx,
                        );

                        next_state = Some(ActiveState::Submitting {
                            cancellation_token,
                            sims_progress: 0,
                            sims_count,
                            bytes_progress: ByteSize::default(),
                            bytes_count,
                        });
                    }
                    KeyCode::Char('n') => {
                        next_state = Some(ActiveState::ViewingList);
                    }
                    _ => {}
                }
            }
            (ActiveState::ReloadingSims, Event::KeyPressed(key_event)) => {
                if let KeyCode::Char('q') = key_event.code {
                    next_state = Some(ActiveState::ViewingList);
                }
            }
            (
                ActiveState::ReloadingSims,
                Event::SimsReloaded(mut simulations),
            ) => {
                // Copy over selection status.
                for new_sim in &mut simulations {
                    new_sim.selected = self
                        .simulations
                        .iter()
                        .find(|sim| sim.dir == new_sim.dir)
                        .is_none_or(|sim| sim.selected);
                }

                self.simulations = simulations;
                next_state = Some(ActiveState::ViewingList);
            }
            (
                ActiveState::Submitting {
                    cancellation_token, ..
                },
                Event::KeyPressed(key_event),
            ) => {
                if let KeyCode::Char('q') = key_event.code {
                    cancellation_token.cancel();

                    // TODO: should we ask for confirmation?
                    next_state = Some(ActiveState::ViewingList);
                }
            }
            (
                ActiveState::Submitting { bytes_progress, .. },
                Event::FileUploaded(size),
            ) => {
                *bytes_progress += size;
            }
            (
                ActiveState::Submitting {
                    sims_progress,
                    sims_count,
                    ..
                },
                Event::SimSubmitted { internal_id, res },
            ) => {
                if let Some(sim_params) = self
                    .simulations
                    .iter_mut()
                    .find(|sim_params| sim_params.internal_id == internal_id)
                {
                    let state = match res {
                        Ok(sim) => SubmissionState::Sent {
                            id: sim.id.clone(),
                            browser_url: sim.browser_url.clone(),
                        },
                        Err(err) => {
                            SubmissionState::Error(human_err_report(&err))
                        }
                    };

                    sim_params
                        .update_submission_state(state)
                        .await
                        .wrap_err("updating submission state")?;

                    *sims_progress += 1;

                    if sims_progress >= sims_count {
                        time::sleep(SLEEP_FOR_FEEDBACK * 3).await;

                        next_state = Some(ActiveState::ViewingList);
                    }
                }
            }
            _ => {}
        }

        if let Some(mut next_state) = next_state {
            mem::swap(state, &mut next_state);
        } else {
            mem::swap(state, &mut curr_state);
        }

        Ok(())
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
            Constraint::Percentage(35),
            Constraint::Percentage(65),
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
            ActiveState::ReloadingSims => {
                Batch::render_reloading_sims_popup(area, buf);
            }
            ActiveState::ConfirmExit { .. } => {
                Batch::render_exit_popup(area, buf);
            }
            ActiveState::ConfirmSubmit => {
                Batch::render_submit_confirmation_popup(simulations, area, buf);
            }
            ActiveState::Submitting {
                bytes_count,
                bytes_progress,
                sims_count,
                sims_progress,
                ..
            } => {
                assert!(*bytes_count > ByteSize::default());
                assert!(*sims_count > 0);

                Batch::render_submitting(
                    *bytes_count,
                    *bytes_progress,
                    *sims_count,
                    *sims_progress,
                    area,
                    buf,
                );
            }
            ActiveState::ViewingList | ActiveState::ViewingDetail => {}
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
            has_focus: matches!(state, ActiveState::ViewingDetail),
            is_dimmed: !matches!(
                state,
                ActiveState::ViewingDetail | ActiveState::ViewingList
            ),
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
            .border_style(if matches!(state, ActiveState::ViewingList) {
                STYLE_NORMAL
            } else {
                STYLE_DIMMED
            })
            .style(
                if matches!(
                    state,
                    ActiveState::ViewingList | ActiveState::ViewingDetail
                ) {
                    STYLE_NORMAL
                } else {
                    STYLE_DIMMED
                },
            );

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

    fn render_reloading_sims_popup(area: Rect, buf: &mut Buffer) {
        let area = center(
            area,
            Constraint::Percentage(38),
            Constraint::Length(5), // top and bottom border + content
        );

        let block = Block::bordered()
            .border_set(border::THICK)
            .style(STYLE_ACCENT);

        let paragraph = Paragraph::new(vec![
            Line::default(),
            Line::raw("Reloading").centered(),
            Line::default(),
        ])
        .block(block)
        .wrap(Wrap { trim: false });

        Widget::render(&Clear, area, buf);
        Widget::render(&paragraph, area, buf);
    }

    fn render_submit_confirmation_popup(
        simulations: &[SimulationParams],
        area: Rect,
        buf: &mut Buffer,
    ) {
        let area = center(
            area,
            Constraint::Percentage(38),
            Constraint::Length(6), // top and bottom border + content
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
            Line::from(vec![
                Span::raw("A total of "),
                Span::raw(format!(
                    "{} simulation(s)",
                    simulations
                        .iter()
                        .filter(|sim_params| sim_params.is_submittable())
                        .count(),
                ))
                .style(STYLE_ACCENT),
                Span::raw(" will be submitted."),
            ])
            .centered(),
            Line::raw("Are you sure you want to continue?").centered(),
            Line::default(),
        ])
        .block(block)
        .wrap(Wrap { trim: false });

        Widget::render(&Clear, area, buf);
        Widget::render(&paragraph, area, buf);
    }

    fn render_template(&self, area: Rect, buf: &mut Buffer) {
        let style = if matches!(
            self.state,
            State::Active {
                state: ActiveState::ViewingDetail | ActiveState::ViewingList,
                ..
            }
        ) {
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
            Span::styled("<space>", STYLE_ACCENT),
            ") toggle selection | (".into(),
            Span::styled("ctrl+r", STYLE_ACCENT),
            ") reset submission state | (".into(),
            Span::styled("r", STYLE_ACCENT),
            ") reload from disk | (".into(),
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

    fn render_submitting(
        bytes_count: ByteSize,
        bytes_progress: ByteSize,
        sims_count: usize,
        sims_progress: usize,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let area =
            center(area, Constraint::Percentage(38), Constraint::Length(12));

        Widget::render(&Clear, area, buf);

        let instructions = Line::from(vec![
            Span::raw(" ("),
            Span::styled("q", STYLE_ACCENT),
            Span::raw(") stop "),
        ]);

        Block::bordered()
            .title(
                Line::from(Span::styled(" Submitting ", STYLE_BOLD)).centered(),
            )
            .title_bottom(instructions.centered())
            .border_set(border::THICK)
            .render(area, buf);

        let [upper, lower] = area.layout(
            &Layout::vertical([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .flex(Flex::Center)
            .margin(2),
        );

        #[allow(clippy::cast_precision_loss)]
        Gauge::default()
            .gauge_style(STYLE_ACCENT)
            .block(
                Block::new()
                    .borders(Borders::NONE)
                    .padding(Padding::vertical(1))
                    .title(Line::from("Uploading files").centered()),
            )
            .ratio(bytes_progress.0 as f64 / bytes_count.0 as f64)
            .label(Span::styled(
                format!("{bytes_progress}/{bytes_count}"),
                Style::new().bold(),
            ))
            .render(upper, buf);

        #[allow(clippy::cast_precision_loss)]
        Gauge::default()
            .gauge_style(STYLE_ACCENT)
            .block(
                Block::new()
                    .borders(Borders::NONE)
                    .padding(Padding::vertical(1))
                    .title(Line::from("Creating simulations").centered()),
            )
            .ratio(sims_progress as f64 / sims_count as f64)
            .label(Span::styled(
                format!("{sims_progress}/{sims_count}"),
                Style::new().bold(),
            ))
            .render(lower, buf);
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

impl Widget for &mut Batch {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.is_term_size_not_enough() {
            self.show_min_term_size_notice(area, buf);
            return;
        }

        match self.state {
            State::Init => {}
            State::PickingProject { ref mut state } => {
                Batch::render_state_picking_project(state, area, buf);
            }
            State::Active {
                ref state,
                ref mut sims_list_state,
                ref mut sim_detail_scrollbar_state,
                ..
            } => {
                Batch::render_state_active(
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
        let style = if p.selected {
            STYLE_NORMAL
        } else {
            STYLE_DIMMED
        };

        let mut spans = vec![Span::raw(p.params.name.clone()), Span::raw(" ")];

        match p.submission_state {
            SubmissionState::Ready => {}
            SubmissionState::Sending => {
                spans.push(Span::styled("(sending...) ", STYLE_WARNING));
            }
            SubmissionState::Error(..) => {
                spans.push(Span::styled("(error) ", STYLE_ERROR));
            }
            SubmissionState::Sent { .. } => {
                spans.push(Span::styled("(sent) ", STYLE_SUCCESS));
            }
        }

        if p.files.is_empty() {
            spans.push(Span::styled("(no files) ", STYLE_ERROR));
        }

        ListItem::from(Line::from(spans).style(style))
    }
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
