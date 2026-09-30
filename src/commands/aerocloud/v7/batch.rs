use crate::{
    aerocloud::{
        Client,
        types::{ProjectV7, SimulationV7},
    },
    commands::aerocloud::v7::batch::{
        log_view::{LogView, LogViewState},
        project_picker::{
            ProjectPicker, ProjectPickerState, refresh_projects_in_background,
        },
        simulation_detail::{SimulationDetail, SimulationDetailState},
        simulation_list::{SimulationList, SimulationListState},
        simulation_params::{SimulationParams, SubmissionState},
        submit::submit_batch_in_background,
    },
    commands::aerocloud::v7::model_submission::ModelSubmitter,
    fmt::human_err_report,
    tracing::LogBuffer,
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
    macros::{constraints, line, span, text},
    prelude::Color,
    style::Style,
    symbols::border,
    text::Text,
    widgets::{
        Block, Borders, Clear, Gauge, Padding, Paragraph, StatefulWidget, Widget,
        Wrap,
    },
};
use std::{
    borrow::Cow,
    mem,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::mpsc, time};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod log_view;
mod project_picker;
mod simulation_detail;
mod simulation_list;
mod simulation_params;
mod submit;

// Made using https://budavariam.github.io/asciiart-text/multi variant `ANSI Shadow`
const LOGO_ASCII_ART: &str = include_str!("../../../aerocloud/logo.txt");

const COLOR_ACCENT: Color = Color::Rgb(0xff, 0xbc, 0x00);

const STYLE_NORMAL: Style = Style::new();
const STYLE_DIMMED: Style = Style::new().dim();
const STYLE_BOLD: Style = Style::new().bold();
const STYLE_ACCENT: Style = Style::new().fg(COLOR_ACCENT).bold();
const STYLE_SUCCESS: Style = Style::new().green().bold();
const STYLE_ERROR: Style = Style::new().red().bold();
const STYLE_WARNING: Style = Style::new().yellow().bold();

const MIN_TERM_SIZE: Size = Size::new(110, 38);

const SLEEP_FOR_FEEDBACK: Duration = Duration::from_millis(100);

/// Minimum interval between redraws caused by new log lines.
const LOG_REDRAW_INTERVAL: Duration = Duration::from_millis(100);

pub async fn run(
    api_client: &Client,
    submitter: &ModelSubmitter,
    root_dir: Option<&Path>,
) -> eyre::Result<()> {
    let sims = if let Some(root_dir) = root_dir {
        let sims =
            SimulationParams::many_from_root_dir(api_client, root_dir).await?;

        if sims.is_empty() {
            tracing::error!("no simulations found in `{}`", root_dir.display());

            return Ok(());
        }

        sims
    } else {
        vec![]
    };

    let logs = crate::tracing::log_buffer();

    let mut app = Batch::new(
        api_client.clone(),
        submitter.clone(),
        root_dir.map(ToOwned::to_owned),
        sims,
        logs,
    );

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal).await;

    ratatui::restore();

    result
}

pub fn refresh_sims_in_background(
    client: Client,
    root_dir: &Path,
    tx: mpsc::Sender<Event>,
) {
    let root_dir = root_dir.to_owned();

    tokio::spawn(async move {
        // NOTE: sleep so that reloading popup is shown and user has visual feedback on the
        // operation.
        time::sleep(SLEEP_FOR_FEEDBACK).await;

        let res = SimulationParams::many_from_root_dir(&client, &root_dir).await;
        tx.send(Event::SimsReloaded(res)).await?;

        Ok::<(), eyre::Report>(())
    });
}

#[derive(Debug)]
struct Batch {
    client: Client,
    submitter: ModelSubmitter,

    running: bool,
    term_size: Size,

    root_dir: Option<PathBuf>,
    simulations: Vec<SimulationParams>,
    logs: Option<Arc<LogBuffer>>,

    state: State,
}

#[derive(Debug, Clone, Default)]
enum ActiveState {
    #[default]
    ViewingList,
    ViewingDetail,
    ConfirmExit {
        prev: Box<Self>,
    },
    ConfirmSubmit,
    ReloadingSims,
    ReloadingSimsFailed(String),
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
        sims_list_state: SimulationListState,
        sim_detail_state: SimulationDetailState,
        /// Overlay on top of any `ActiveState`, so that e.g. submission keeps progressing.
        log_view: Option<LogViewState>,
    },
}

#[derive(Debug)]
pub enum Event {
    KeyPressed(crossterm::event::KeyEvent),
    TerminalResized(Size),
    ProjectsLoading,
    ProjectsUpdated(eyre::Result<Vec<ProjectV7>>),
    ProjectSelected(Box<ProjectV7>),
    UploadProgressed(i64),
    SimsReloaded(eyre::Result<Vec<SimulationParams>>),
    SimSubmitted {
        internal_id: Uuid,
        res: eyre::Result<Box<SimulationV7>, eyre::Report>,
    },
    LogsUpdated,
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

async fn forward_log_updates(
    logs: Arc<LogBuffer>,
    tx: mpsc::Sender<Event>,
) -> eyre::Result<()> {
    loop {
        logs.changed().await;
        tx.send(Event::LogsUpdated).await?;

        // NOTE: coalesce bursts of lines into a single redraw.
        time::sleep(LOG_REDRAW_INTERVAL).await;
    }
}

impl Batch {
    fn new(
        client: Client,
        submitter: ModelSubmitter,
        root_dir: Option<PathBuf>,
        simulations: Vec<SimulationParams>,
        logs: Option<Arc<LogBuffer>>,
    ) -> Self {
        Self {
            state: State::Init,
            running: false,
            term_size: Size::default(),
            root_dir,
            simulations,
            logs,
            client,
            submitter,
        }
    }

    async fn run(&mut self, terminal: &mut DefaultTerminal) -> eyre::Result<()> {
        self.term_size = terminal.size().wrap_err("getting term size")?;
        self.running = true;

        let (event_tx, mut event_rx) = mpsc::channel(10);

        tokio::spawn(handle_term_events(event_tx.clone()));

        if let Some(ref logs) = self.logs {
            tokio::spawn(forward_log_updates(Arc::clone(logs), event_tx.clone()));
        }

        if matches!(self.state, State::Init) {
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

        if matches!(event, Event::Exit) {
            self.immediate_exit();
            return Ok(());
        }

        // NOTE: only needs a redraw.
        if matches!(event, Event::LogsUpdated) {
            return Ok(());
        }

        match self.state {
            State::Init => {}
            State::PickingProject { ref mut state } => {
                if let Event::ProjectSelected(project) = event {
                    self.state = State::Active {
                        project,
                        state: ActiveState::ViewingList,
                        sims_list_state: SimulationListState::default(),
                        sim_detail_state: SimulationDetailState::default(),
                        log_view: None,
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
            ref mut sim_detail_state,
            ref mut log_view,
        } = self.state
        else {
            return Ok(());
        };

        // NOTE: while logs are shown they capture all key presses, every other event is still
        // handled by the underlying state.
        if let Event::KeyPressed(key_event) = event
            && let Some(ref logs) = self.logs
        {
            if let Some(view) = log_view {
                if !view.handle_key(key_event, logs) {
                    *log_view = None;
                }

                return Ok(());
            }

            if key_event.code == KeyCode::Char('l') {
                *log_view = Some(LogViewState::default());

                return Ok(());
            }
        }

        let mut curr_state = mem::take(state);
        let mut next_state: Option<ActiveState> = None;

        match (&mut curr_state, event) {
            (ActiveState::ViewingList, Event::KeyPressed(key_event)) => {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char(' '), _)
                        if let Some(idx) = sims_list_state.selected()
                            && let Some(sim) = self.simulations.get_mut(idx) =>
                    {
                        sim.selected = !sim.selected;
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL)
                        if let Some(idx) = sims_list_state.selected()
                            && let Some(sim) = self.simulations.get_mut(idx)
                            && let Err(err) =
                                sim.reset_submission_state().await =>
                    {
                        tracing::error!(
                            "failed to flush submission state for sim in dir `{}`: {err:?}",
                            sim.dir.display()
                        );
                    }
                    (KeyCode::Char('o'), KeyModifiers::CONTROL)
                        if self
                            .simulations
                            .iter()
                            .any(SimulationParams::is_submittable) =>
                    {
                        next_state = Some(ActiveState::ConfirmSubmit);
                    }
                    (KeyCode::Char('r'), _) => {
                        if let Some(root_dir) = self.root_dir.as_ref() {
                            refresh_sims_in_background(
                                self.client.clone(),
                                root_dir,
                                tx.clone(),
                            );
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
                        sim_detail_state.reset();
                    }
                    (KeyCode::Down, _) => {
                        sims_list_state.select_next();
                        sim_detail_state.reset();
                    }
                    (KeyCode::Left, _) => {
                        sims_list_state.pan_left();
                    }
                    (KeyCode::Right, _) => {
                        sims_list_state.pan_right();
                    }
                    (KeyCode::Tab, _) => {
                        next_state = Some(ActiveState::ViewingDetail);
                    }
                    _ => {}
                }
            }
            (ActiveState::ViewingDetail, Event::KeyPressed(key_event)) => {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char(' '), _)
                        if let Some(idx) = sims_list_state.selected()
                            && let Some(sim) = self.simulations.get_mut(idx) =>
                    {
                        sim.selected = !sim.selected;
                    }
                    (KeyCode::Char('r'), KeyModifiers::CONTROL)
                        if let Some(idx) = sims_list_state.selected()
                            && let Some(sim) = self.simulations.get_mut(idx)
                            && let Err(err) =
                                sim.reset_submission_state().await =>
                    {
                        tracing::error!(
                            "failed to flush submission state for sim in dir `{}`: {err:?}",
                            sim.dir.display()
                        );
                    }
                    (KeyCode::Char('r'), _) => {
                        if let Some(root_dir) = self.root_dir.as_ref() {
                            refresh_sims_in_background(
                                self.client.clone(),
                                root_dir,
                                tx.clone(),
                            );
                        }

                        next_state = Some(ActiveState::ReloadingSims);
                    }
                    (KeyCode::Char('o'), KeyModifiers::CONTROL)
                        if self
                            .simulations
                            .iter()
                            .any(SimulationParams::is_submittable) =>
                    {
                        next_state = Some(ActiveState::ConfirmSubmit);
                    }
                    (KeyCode::Esc, _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        next_state = Some(ActiveState::ConfirmExit {
                            prev: Box::new(ActiveState::ViewingDetail),
                        });
                    }
                    (KeyCode::Up, KeyModifiers::SHIFT) => {
                        sims_list_state.select_previous();
                        sim_detail_state.reset();
                    }
                    (KeyCode::Down, KeyModifiers::SHIFT) => {
                        sims_list_state.select_next();
                        sim_detail_state.reset();
                    }
                    (KeyCode::Up, _) => {
                        sim_detail_state.scroll_up();
                    }
                    (KeyCode::Down, _) => {
                        sim_detail_state.scroll_down();
                    }
                    (KeyCode::Left, _) => {
                        sim_detail_state.pan_left();
                    }
                    (KeyCode::Right, _) => {
                        sim_detail_state.pan_right();
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
                            &self.submitter,
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
            (ActiveState::ReloadingSims, Event::KeyPressed(key_event))
                if key_event.code == KeyCode::Char('q') =>
            {
                next_state = Some(ActiveState::ViewingList);
            }
            (
                ActiveState::ReloadingSims,
                Event::SimsReloaded(Ok(mut simulations)),
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
            (ActiveState::ReloadingSims, Event::SimsReloaded(Err(err))) => {
                next_state = Some(ActiveState::ReloadingSimsFailed(
                    human_err_report(&err),
                ));
            }
            (
                ActiveState::ReloadingSimsFailed(..),
                Event::KeyPressed(key_event),
            ) if key_event.code == KeyCode::Char('q') => {
                next_state = Some(ActiveState::ViewingList);
            }
            (
                ActiveState::Submitting {
                    cancellation_token, ..
                },
                Event::KeyPressed(key_event),
            ) if key_event.code == KeyCode::Char('q') => {
                cancellation_token.cancel();

                // TODO: should we ask for confirmation?
                next_state = Some(ActiveState::ViewingList);
            }
            (
                ActiveState::Submitting { bytes_progress, .. },
                Event::UploadProgressed(delta),
            ) => {
                *bytes_progress =
                    ByteSize::b(bytes_progress.0.saturating_add_signed(delta));
            }
            (
                ActiveState::Submitting {
                    sims_progress,
                    sims_count,
                    bytes_count,
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
                            // NOTE: its upload progress has been rolled back already.
                            *bytes_count = ByteSize::b(
                                bytes_count
                                    .0
                                    .saturating_sub(sim_params.files_size().0),
                            );

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

    const fn immediate_exit(&mut self) {
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

    #[allow(clippy::too_many_arguments)]
    fn render_state_active(
        state: &ActiveState,
        simulations: &[SimulationParams],
        sims_list_state: &mut SimulationListState,
        sim_detail_state: &mut SimulationDetailState,
        log_view: Option<(&mut LogViewState, &LogBuffer)>,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let layout = Layout::horizontal([
            Constraint::Percentage(35),
            Constraint::Percentage(65),
        ])
        .vertical_margin(1)
        .horizontal_margin(2);

        // NOTE: leave the row above the bottom border free for the instructions.
        let panes_area = Rect {
            height: area.height.saturating_sub(1),
            ..area
        };

        let [left_area, right_area] = panes_area.layout(&layout);

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
            sim_detail_state,
            right_area,
            buf,
        );

        match state {
            ActiveState::ReloadingSims => {
                Self::render_reloading_sims_popup(area, buf);
            }
            ActiveState::ReloadingSimsFailed(error) => {
                Self::render_reloading_sims_failed_popup(area, buf, error);
            }
            ActiveState::ConfirmExit { .. } => {
                Self::render_exit_popup(area, buf);
            }
            ActiveState::ConfirmSubmit => {
                Self::render_submit_confirmation_popup(simulations, area, buf);
            }
            ActiveState::Submitting {
                bytes_count,
                bytes_progress,
                sims_count,
                sims_progress,
                ..
            } => {
                assert!(*bytes_count >= ByteSize::default());
                assert!(*sims_count > 0);

                Self::render_submitting(
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

        if let Some((log_view_state, logs)) = log_view {
            let area = center(
                area,
                Constraint::Percentage(90),
                Constraint::Percentage(85),
            );

            StatefulWidget::render(&LogView { logs }, area, buf, log_view_state);
        }
    }

    fn render_sim_detail(
        state: &ActiveState,
        simulations: &[SimulationParams],
        sims_list_state: &SimulationListState,
        sim_detail_state: &mut SimulationDetailState,
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

        StatefulWidget::render(&detail, area, buf, sim_detail_state);
    }

    fn render_sims_list(
        state: &ActiveState,
        simulations: &[SimulationParams],
        sims_list_state: &mut SimulationListState,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let list = SimulationList {
            has_focus: matches!(state, ActiveState::ViewingList),
            is_dimmed: !matches!(
                state,
                ActiveState::ViewingList | ActiveState::ViewingDetail
            ),
            sims: simulations,
        };

        StatefulWidget::render(&list, area, buf, sims_list_state);
    }

    fn render_exit_popup(area: Rect, buf: &mut Buffer) {
        let area = center(
            area,
            Constraint::Percentage(38),
            Constraint::Length(5), // top and bottom border + content
        );

        let instructions = line![
            " (",
            span!(STYLE_ERROR; "y"),
            ") yes | (",
            span!(STYLE_ERROR; "n"),
            ") no ",
        ];

        let block = Block::bordered()
            .title(line![span!(STYLE_BOLD; " Confirmation ")].centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK)
            .style(STYLE_ERROR);

        let paragraph = Paragraph::new(text![
            "",
            line!["Are you sure you want to exit?"].centered(),
            "",
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

        let paragraph =
            Paragraph::new(text!["", line!["Reloading"].centered(), "",])
                .block(block)
                .wrap(Wrap { trim: false });

        Widget::render(&Clear, area, buf);
        Widget::render(&paragraph, area, buf);
    }

    fn render_reloading_sims_failed_popup(
        area: Rect,
        buf: &mut Buffer,
        error: &str,
    ) {
        let lines = {
            let mut l = vec![line![]];

            for line in error.lines() {
                l.push(line![line]);
            }

            l.push(line![]);

            l
        };

        let area = center(
            area,
            Constraint::Percentage(55),
            Constraint::Length(u16::try_from(lines.len()).unwrap_or(5) + 2), // top and bottom border + content
        );

        let block = Block::bordered()
            .title(
                line![span!(STYLE_BOLD; " Failed to reload config ")].centered(),
            )
            .title_bottom(line![" (q) close and continue "].centered())
            .border_set(border::THICK)
            .style(STYLE_ERROR);

        let paragraph = Paragraph::new(lines).block(block);

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

        let instructions = line![
            " (",
            span!(STYLE_ACCENT; "y"),
            ") yes | (",
            span!(STYLE_ACCENT; "n"),
            ") no ",
        ];

        let block = Block::bordered()
            .title(line![span!(STYLE_BOLD; " Launching batch ")].centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let paragraph = Paragraph::new(text![
            "",
            line![
                "A total of ",
                span!(STYLE_ACCENT; format!(
                    "{} simulation(s)",
                    simulations
                        .iter()
                        .filter(|sim_params| sim_params.is_submittable())
                        .count(),
                )),
                " will be submitted.",
            ]
            .centered(),
            line!["Are you sure you want to continue?"].centered(),
            "",
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

        let instructions_upper = line![
            " (",
            span!(STYLE_ACCENT; "tab"),
            ") cycle list<->detail | (",
            span!(STYLE_ACCENT; "<space>"),
            ") toggle selection | (",
            span!(STYLE_ACCENT; "ctrl+r"),
            ") reset submission state ",
        ];

        let instructions_lower = line![
            " (",
            span!(STYLE_ACCENT; "r"),
            ") reload from disk | (",
            span!(STYLE_ACCENT; "ctrl+o"),
            ") submit batch | (",
            span!(STYLE_ACCENT; "l"),
            ") logs | (",
            span!(STYLE_ACCENT; "esc"),
            ") quit ",
        ];

        let block = Block::bordered()
            .title(line![span!(STYLE_BOLD; title)].centered())
            .title_bottom(instructions_lower.style(style).centered())
            .border_set(border::THICK);

        Widget::render(&block, area, buf);

        // NOTE: a block can only have one line of bottom titles, so render the other on the
        // row right above the border.
        if area.height >= 3 {
            let upper_area = Rect::new(
                area.x + 1,
                area.bottom() - 2,
                area.width.saturating_sub(2),
                1,
            );

            instructions_upper
                .style(style)
                .centered()
                .render(upper_area, buf);
        }
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

        let instructions =
            line![" (", span!(STYLE_ACCENT; "q"), ") stop ",].centered();

        Block::bordered()
            .title(line![span!(STYLE_BOLD; " Submitting ")].centered())
            .title_bottom(instructions)
            .border_set(border::THICK)
            .render(area, buf);

        let [upper, lower] = area.layout(
            &Layout::vertical(constraints![==50%, ==50%])
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
                    .title(line!["Uploading files"].centered()),
            )
            .ratio(if bytes_count.0 == 0 {
                1.0
            } else {
                bytes_progress.0 as f64 / bytes_count.0 as f64
            })
            .label(span!(STYLE_BOLD; format!("{bytes_progress}/{bytes_count}")))
            .render(upper, buf);
        darken_label_over_filled_gauge(upper, buf);

        #[allow(clippy::cast_precision_loss)]
        Gauge::default()
            .gauge_style(STYLE_ACCENT)
            .block(
                Block::new()
                    .borders(Borders::NONE)
                    .padding(Padding::vertical(1))
                    .title(line!["Creating simulations"].centered()),
            )
            .ratio(sims_progress as f64 / sims_count as f64)
            .label(span!(STYLE_BOLD; format!("{sims_progress}/{sims_count}")))
            .render(lower, buf);
        darken_label_over_filled_gauge(lower, buf);
    }

    const fn is_term_size_not_enough(&self) -> bool {
        self.term_size.width < MIN_TERM_SIZE.width
            || self.term_size.height < MIN_TERM_SIZE.height
    }

    fn show_min_term_size_notice(&self, area: Rect, buf: &mut Buffer) {
        let paragraph = Paragraph::new(text![
            "",
            line![format!(
                "Terminal size is too small ({}x{}).",
                self.term_size.width, self.term_size.height,
            )]
            .centered(),
            line![format!(
                "Need at least {}x{}",
                MIN_TERM_SIZE.width, MIN_TERM_SIZE.height
            )]
            .centered(),
            "",
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
                ref mut sim_detail_state,
                ref mut log_view,
                ..
            } => {
                Batch::render_state_active(
                    state,
                    &self.simulations,
                    sims_list_state,
                    sim_detail_state,
                    log_view.as_mut().zip(self.logs.as_deref()),
                    area,
                    buf,
                );
            }
        }

        self.render_template(area, buf);
    }
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

/// `Gauge` swaps fg/bg for label cells over the filled part, leaving the text in
/// the terminal's default fg (usually white) on top of the accent color, which is
/// hard to read. Those are the only cells with an accent background, so force
/// their text to black.
fn darken_label_over_filled_gauge(area: Rect, buf: &mut Buffer) {
    for pos in area.positions() {
        let cell = &mut buf[pos];

        if cell.bg == COLOR_ACCENT {
            cell.set_fg(Color::Black);
        }
    }
}
