use comfy_table::{ContentArrangement, Table, presets::UTF8_FULL};

pub fn new_dynamic_table() -> Table {
    let mut t = Table::new();

    t.set_content_arrangement(ContentArrangement::Dynamic)
        .load_style(UTF8_FULL.with_rounded_corners());

    t
}
