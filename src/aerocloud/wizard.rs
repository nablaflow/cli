use crate::aerocloud::{
    Client,
    extra_types::CreateSimulationV7ParamsFromJson,
    types::ProjectV7,
    wizard::{
        project_picker::{ProjectPicker, WidgetResult as ProjectPickerResult},
        simulation_detail::SimulationDetail,
        simulation_params::{ModelState, SimulationParams},
    },
};
use color_eyre::eyre;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect, Spacing},
    style::Style,
    symbols::border,
    text::{Line, Span, Text},
    widgets::{
        Block, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};
use std::{borrow::Cow, path::Path, time::Duration};

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

pub async fn run(client: &Client, root_dir: Option<&Path>) -> eyre::Result<()> {
    let sims = if let Some(root_dir) = root_dir {
        SimulationParams::many_from_root_dir(root_dir)?
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
    event_stream: EventStream,

    simulations: Vec<SimulationParams>,

    state: State,
}

#[derive(Debug, Clone)]
enum ActiveState {
    ViewingList,
    ViewingDetail,
    ConfirmExit { prev: Box<ActiveState> },
}

impl ActiveState {
    fn toggle_viewing_focus(&mut self) {
        match self {
            Self::ViewingList => *self = Self::ViewingDetail,
            Self::ViewingDetail => *self = Self::ViewingList,
            Self::ConfirmExit { .. } => {}
        }
    }
}

#[derive(Debug)]
enum State {
    PickingProject {
        picker: ProjectPicker,
    },
    Active {
        state: ActiveState,
        project: ProjectV7,
        sims_list_state: ListState,
        sim_detail_scrollbar_state: ScrollbarState,
    },
}

impl Wizard {
    const FRAMES_PER_SECOND: f32 = 10.0;

    fn new(client: Client, simulations: Vec<SimulationParams>) -> Self {
        Self {
            state: State::PickingProject {
                picker: ProjectPicker::new(client.clone()),
            },
            running: false,
            simulations,
            event_stream: EventStream::default(),
            client,
        }
    }

    async fn run(&mut self, terminal: &mut DefaultTerminal) -> eyre::Result<()> {
        let mut drawing_interval = tokio::time::interval(
            Duration::from_secs_f32(1.0 / Self::FRAMES_PER_SECOND),
        );

        self.running = true;

        while self.running {
            terminal.draw(|frame| self.draw(frame))?;

            let Some(event) = tokio::select! (
                _ = drawing_interval.tick() => {
                    terminal.draw(|frame| self.draw(frame))?;
                    None
                },
                Some(Ok(event)) = self.event_stream.next() => Some(event),
            ) else {
                continue;
            };

            match self.state {
                State::PickingProject { .. } => {
                    self.handle_event_state_picking_project(&event);
                }
                State::Active { .. } => self.handle_event_state_active(&event),
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_event_state_picking_project(&mut self, event: &Event) {
        let State::PickingProject { ref mut picker } = self.state else {
            return;
        };

        match picker.handle_event(event) {
            Some(ProjectPickerResult::Exit) => self.immediate_exit(),
            Some(ProjectPickerResult::Selected(project)) => {
                if self.simulations.is_empty() {
                    self.simulations.push(build_new_sim());
                }

                self.state = State::Active {
                    project,
                    state: ActiveState::ViewingList,
                    sims_list_state: ListState::default().with_selected(Some(0)),
                    sim_detail_scrollbar_state: ScrollbarState::default(),
                };
            }
            _ => {}
        }
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

        let Event::Key(key_event) = event else {
            return;
        };

        if key_event.kind != KeyEventKind::Press {
            return;
        }

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
        }
    }

    fn immediate_exit(&mut self) {
        self.running = false;
    }

    fn render_state_picking_project(
        picker: &ProjectPicker,
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

        Widget::render(picker, lower, buf);
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

        if let ActiveState::ConfirmExit { .. } = state {
            Wizard::render_exit_popup(area, buf);
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
            Constraint::Percentage(40),
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
            Span::styled("ctrl+l", STYLE_ACCENT),
            ") launch batch | (".into(),
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
}

fn build_new_sim() -> SimulationParams {
    SimulationParams {
        params: CreateSimulationV7ParamsFromJson::default(),
        model_state: ModelState::Pending,
        files: vec![],
    }
}

impl Widget for &mut Wizard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.state {
            State::PickingProject { ref picker } => {
                Wizard::render_state_picking_project(picker, area, buf);
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
