use color_eyre::eyre;
use itertools::Itertools;
use std::fmt::Display;

pub static NOT_AVAILABLE: &str = "n/a";

pub fn link_with_text(text: impl Display, url: impl Display) -> String {
    format!("\u{1b}]8;;{url}\u{1b}\\{text}\u{1b}]8;;\u{1b}\\")
}

pub fn link(url: impl Display) -> String {
    link_with_text("[Open in Browser ↗ ]", url)
}

pub fn human_err_report(report: &eyre::Report) -> String {
    let pad = (report.chain().count() + 1) / 10;

    report
        .chain()
        .enumerate()
        .map(|(idx, cause)| {
            format!("{cause}")
                .split('\n')
                .enumerate()
                .map(|(line_idx, line)| {
                    if line_idx == 0 {
                        format!("{idx:pad$}. {line}", idx = idx + 1)
                    } else {
                        format!("{empty:pad$}   {line}", empty = "")
                    }
                })
                .join("\n")
        })
        .join("\n")
}
