use std::fmt::Display;

pub fn display_optional(v: Option<impl Display>) -> String {
    if let Some(v) = v {
        format!("{v}")
    } else {
        "n/a".into()
    }
}
