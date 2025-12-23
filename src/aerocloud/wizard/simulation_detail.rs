use crate::aerocloud::wizard::{
    STYLE_ACCENT, STYLE_BOLD, STYLE_DIMMED, STYLE_ERROR, STYLE_NORMAL,
    STYLE_SUCCESS, STYLE_WARNING,
    simulation_params::{FileParams, SimulationParams, SubmissionState},
};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    symbols::border,
    text::{Line, Span, Text},
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget,
    },
};
use std::borrow::Cow;

pub struct SimulationDetail<'a> {
    pub focus: bool,
    pub sim: Option<&'a SimulationParams>,
}

impl<'a> SimulationDetail<'a> {
    const GENERIC_TITLE: &'static str = " Params ";

    fn block(&self) -> Block<'_> {
        Block::bordered()
            .title(Line::from(self.block_title()).centered())
            .border_set(border::PLAIN)
            .border_style(if self.focus {
                STYLE_NORMAL
            } else {
                STYLE_DIMMED
            })
    }

    fn block_title(&self) -> Cow<'_, str> {
        let Some(sim) = self.sim else {
            return Self::GENERIC_TITLE.into();
        };

        format!(" {} ", sim.params.name).into()
    }

    fn submission_state(sim: &'a SimulationParams, lines: &mut Vec<Line<'a>>) {
        match sim.submission_state {
            SubmissionState::Ready | SubmissionState::Sending => {
                return;
            }
            SubmissionState::Error(ref err) => {
                lines.push(Line::raw("Error on submission:").style(STYLE_ERROR));
                lines.push(Line::default());
                lines.push(Line::raw(err.clone()).style(STYLE_ERROR));
            }
            SubmissionState::Sent => {
                lines.push(Line::raw("Sent with success.").style(STYLE_SUCCESS));
            }
        }

        lines.push(Line::default());
    }

    fn general_lines(sim: &'a SimulationParams, lines: &mut Vec<Line<'a>>) {
        if !sim.selected {
            lines.push(Line::from(vec![Span::styled(
                "Not selected, will not be submitted.",
                STYLE_WARNING,
            )]));
            lines.push(Line::default());
        }

        lines.push(Line::from(vec![
            Span::styled("Revision: ", STYLE_BOLD),
            Span::styled(sim.params.revision_or_placeholder(), STYLE_ACCENT),
        ]));

        lines.push(Line::default());

        lines.push(Line::from(vec![
            Span::styled("Quality: ", STYLE_BOLD),
            Span::styled(sim.params.quality.to_string(), STYLE_ACCENT),
        ]));

        lines.push(Line::default());

        lines.push(Line::from(vec![
            Span::styled("Yaw angles: ", STYLE_BOLD),
            Span::styled(
                sim.params
                    .yaw_angles
                    .iter()
                    .map(|yaw_angle| format!("{yaw_angle}°"))
                    .join(", "),
                STYLE_ACCENT,
            ),
        ]));

        lines.push(Line::default());

        lines.push(Line::from(vec![
            Span::styled("Fluid: ", STYLE_BOLD),
            Span::styled(sim.params.fluid.to_string(), STYLE_ACCENT),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Speed: ", STYLE_BOLD),
            Span::styled(format!("{} m/s", sim.params.fluid_speed), STYLE_ACCENT),
        ]));

        lines.push(Line::default());

        lines.push(Line::from(vec![
            Span::styled("Has ground: ", STYLE_BOLD),
            Span::styled(bool_to_human(sim.params.has_ground), STYLE_ACCENT),
        ]));

        if sim.params.has_ground {
            lines.push(Line::from(vec![
                Span::styled("Offset: ", STYLE_BOLD),
                Span::styled(
                    format!("{} m", sim.params.ground_offset),
                    STYLE_ACCENT,
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Moving: ", STYLE_BOLD),
                Span::styled(
                    bool_to_human(sim.params.is_ground_moving),
                    STYLE_ACCENT,
                ),
            ]));
        }
    }

    fn files_lines(sim: &'a SimulationParams, lines: &mut Vec<Line<'a>>) {
        let files = &sim.files;

        if files.is_empty() {
            lines.push(Line::styled(
                "No files present! The simulation cannot be submitted",
                STYLE_ERROR,
            ));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Files ", STYLE_BOLD),
                Span::styled(format!("({})", files.len()), STYLE_ACCENT),
                Span::styled(":", STYLE_BOLD),
            ]));
        }

        for file in files {
            lines.push(Line::default());

            lines.push(Line::from(vec![
                Span::raw("  - "),
                Span::styled("Name: ", STYLE_BOLD),
                Span::styled(
                    format!("{}", file.path.file_name().unwrap().display()),
                    STYLE_ACCENT,
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("Unit: ", STYLE_BOLD),
                Span::styled(format!("{}", file.params.unit), STYLE_ACCENT),
            ]));

            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("Rotation: ", STYLE_BOLD),
                if let Some(rotation) = &file.params.rotation {
                    Span::styled(
                        format!("{:?} (quaternion)", rotation.0),
                        STYLE_ACCENT,
                    )
                } else {
                    Span::styled("none", STYLE_ACCENT)
                },
            ]));

            lines.push(Line::default());

            Self::parts_lines(file, lines);
        }
    }

    fn parts_lines(file: &'a FileParams, lines: &mut Vec<Line<'a>>) {
        let parts = &file.params.parts;

        if parts.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    "No parts configured. What the file contains will be used as is.",
                    STYLE_WARNING
                ),
            ]));

            return;
        }

        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("Parts ", STYLE_BOLD),
            Span::styled(format!("({})", parts.len()), STYLE_ACCENT),
            Span::styled(":", STYLE_BOLD),
        ]));

        for (name, part) in parts {
            lines.push(Line::default());

            lines.push(Line::from(vec![
                Span::raw("      - "),
                Span::styled("Name: ", STYLE_BOLD),
                Span::styled(name, STYLE_ACCENT),
            ]));

            lines.push(Line::from(vec![
                Span::raw("        "),
                Span::styled("Rolling: ", STYLE_BOLD),
                Span::styled(
                    bool_to_human(part.rolling.unwrap_or(false)),
                    STYLE_ACCENT,
                ),
            ]));

            let is_porous = part.is_porous.unwrap_or(false);

            lines.push(Line::from(vec![
                Span::raw("        "),
                Span::styled("Porous: ", STYLE_BOLD),
                Span::styled(bool_to_human(is_porous), STYLE_ACCENT),
            ]));

            if is_porous {
                lines.push(Line::from(vec![
                    Span::raw("        "),
                    Span::styled("Darcy coeff: ", STYLE_BOLD),
                    if let Some(darcy_coeff) = &part.darcy_coeff {
                        Span::styled(format!("{darcy_coeff}"), STYLE_ACCENT)
                    } else {
                        Span::styled(
                            "<unspecified> (required when part is marked as porous)",
                            STYLE_ERROR
                        )
                    }
                ]));

                lines.push(Line::from(vec![
                    Span::raw("        "),
                    Span::styled("Forchheimer coeff: ", STYLE_BOLD),
                    if let Some(forchheimer_coeff) = &part.forchheimer_coeff {
                        Span::styled(format!("{forchheimer_coeff}"), STYLE_ACCENT)
                    } else {
                        Span::styled(
                            "<unspecified> (required when part is marked as porous)",
                            STYLE_ERROR
                        )
                    }
                ]));
            }
        }
    }
}

impl StatefulWidget for &SimulationDetail<'_> {
    type State = ScrollbarState;

    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        scrollbar_state: &mut Self::State,
    ) {
        let block = self.block();

        let Some(sim) = self.sim else {
            Paragraph::new(Text::raw("Select a simulation to display."))
                .block(block)
                .render(area, buf);
            return;
        };

        let mut lines = Vec::with_capacity(10);

        SimulationDetail::submission_state(sim, &mut lines);

        SimulationDetail::general_lines(sim, &mut lines);

        lines.push(Line::default());

        SimulationDetail::files_lines(sim, &mut lines);

        *scrollbar_state = scrollbar_state.content_length(lines.len());

        Paragraph::new(lines)
            .scroll((u16::try_from(scrollbar_state.get_position()).unwrap(), 0))
            .block(block)
            .render(area, buf);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);

        StatefulWidget::render(scrollbar, area, buf, scrollbar_state);
    }
}

fn bool_to_human(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}
