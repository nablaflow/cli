use comfy_table::{
    ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
};

pub fn new_dynamic_table() -> Table {
    let mut t = Table::new();

    t.set_content_arrangement(ContentArrangement::Dynamic)
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);
    t
}
