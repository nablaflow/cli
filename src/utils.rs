use comfy_table::{ContentArrangement, Table, presets::UTF8_FULL};

pub fn new_dynamic_table() -> Table {
    let mut t = Table::new();

    t.set_content_arrangement(ContentArrangement::Dynamic)
        .load_style(UTF8_FULL.with_rounded_corners());

    // Pseudo-terminals without a size (e.g. Nix builders, `docker run -t`) report 0
    // columns, which would squeeze every column down to a single character.
    if t.width() == Some(0) {
        t.force_no_tty();
    }

    t
}
