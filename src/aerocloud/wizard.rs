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
        Block, HighlightSpacing, List, ListItem, ListState, StatefulWidget,
        Widget,
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

#[derive(Debug, Default)]
enum ActiveFocus {
    #[default]
    List,
    Detail,
}

impl ActiveFocus {
    fn toggle(&mut self) {
        match self {
            Self::List => *self = Self::Detail,
            Self::Detail => *self = Self::List,
        }
    }
}

#[derive(Debug)]
enum State {
    PickingProject {
        picker: ProjectPicker,
    },
    Active {
        focus: ActiveFocus,
        project: ProjectV7,
        list_state: ListState,
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
                State::PickingProject { ref mut picker } => {
                    match picker.handle_event(&event) {
                        Some(ProjectPickerResult::Exit) => self.exit(),
                        Some(ProjectPickerResult::Selected(project)) => {
                            if self.simulations.is_empty() {
                                self.simulations.push(build_new_sim());
                            }

                            self.state = State::Active {
                                project,
                                focus: ActiveFocus::List,
                                list_state: ListState::default()
                                    .with_selected(Some(0)),
                            };
                        }
                        _ => {}
                    }
                }
                State::Active {
                    ref focus,
                    ref mut list_state,
                    ..
                } => {
                    match focus {
                        ActiveFocus::List => {
                            if let Event::Key(key_event) = event
                                && key_event.kind == KeyEventKind::Press
                            {
                                match key_event.code {
                                    KeyCode::Up => {
                                        list_state.select_previous();
                                    }
                                    KeyCode::Down => {
                                        list_state.select_next();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        ActiveFocus::Detail => {}
                    }

                    self.handle_event(&event);
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_event(&mut self, event: &Event) {
        if let Event::Key(key_event) = event
            && key_event.kind == KeyEventKind::Press
        {
            if key_event.code == KeyCode::Esc
                || (key_event.code == KeyCode::Char('c')
                    && key_event.modifiers == KeyModifiers::CONTROL)
            {
                self.exit();
            }

            if let State::Active {
                ref mut focus,
                ref mut list_state,
                ..
            } = self.state
            {
                match key_event.code {
                    KeyCode::Tab => {
                        focus.toggle();
                    }
                    KeyCode::Up if key_event.modifiers == KeyModifiers::SHIFT => {
                        list_state.select_previous();
                    }
                    KeyCode::Down
                        if key_event.modifiers == KeyModifiers::SHIFT =>
                    {
                        list_state.select_next();
                    }
                    _ => {}
                }
            }
        }
    }

    fn exit(&mut self) {
        // TODO: ask for confirmation
        self.running = false;
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
                let [upper, lower] = area.layout(
                    &Layout::vertical([Constraint::Min(8), Constraint::Fill(1)])
                        .flex(Flex::Center)
                        .vertical_margin(5)
                        .horizontal_margin(10)
                        .spacing(Spacing::Space(2)),
                );

                Text::from(LOGO_ASCII_ART).centered().render(
                    upper.centered_vertically(Constraint::Ratio(1, 2)),
                    buf,
                );
                picker.render(lower, buf);
            }
            State::Active {
                ref focus,
                ref mut list_state,
                ..
            } => {
                let layout = Layout::horizontal([
                    Constraint::Percentage(30),
                    Constraint::Percentage(70),
                ])
                .vertical_margin(1)
                .horizontal_margin(2);

                let [left_area, right_area] = area.layout(&layout);

                let sims_list_block = Block::bordered()
                    .title(
                        Line::from(format!(
                            "Simulations ({})",
                            self.simulations.len()
                        ))
                        .centered(),
                    )
                    .border_set(border::PLAIN)
                    .style(if let ActiveFocus::List = focus {
                        STYLE_NORMAL
                    } else {
                        STYLE_DIMMED
                    });

                let sims_list = List::new(self.simulations.iter())
                    .block(sims_list_block)
                    .highlight_spacing(HighlightSpacing::Always)
                    .highlight_symbol(">> ")
                    .highlight_style(STYLE_ACCENT);

                StatefulWidget::render(sims_list, left_area, buf, list_state);

                SimulationDetail {
                    focus: matches!(focus, ActiveFocus::Detail),
                    sim: list_state
                        .selected()
                        .and_then(|idx| self.simulations.get(idx)),
                }
                .render(right_area, buf);
            }
        }

        let style = if matches!(self.state, State::Active { .. }) {
            STYLE_NORMAL
        } else {
            STYLE_DIMMED
        };

        let title: Cow<'_, str> =
            if let State::Active { ref project, .. } = self.state {
                format!(" AeroCloud v7 (project: `{}`)", project.name).into()
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

        block.render(area, buf);
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
