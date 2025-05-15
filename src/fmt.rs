use std::fmt::Display;

pub static NOT_AVAILABLE: &str = "n/a";

pub fn link_with_text(text: impl Display, url: impl Display) -> String {
    format!("\u{1b}]8;;{url}\u{1b}\\{text}\u{1b}]8;;\u{1b}\\")
}

pub fn link(url: impl Display) -> String {
    link_with_text("[Open in Browser ↗ ]", url)
}
