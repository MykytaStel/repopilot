pub(crate) fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}
