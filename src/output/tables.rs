use comfy_table::presets::{ASCII_FULL_CONDENSED, UTF8_FULL_CONDENSED};
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table};

pub fn new_table(headers: &[&str], ascii: bool) -> Table {
    let mut table = Table::new();
    table
        .load_preset(if ascii {
            ASCII_FULL_CONDENSED
        } else {
            UTF8_FULL_CONDENSED
        })
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers.iter().map(|header| Cell::new(*header)));
    table
}

pub fn right_align(cell: &str) -> Cell {
    Cell::new(cell).set_alignment(CellAlignment::Right)
}
