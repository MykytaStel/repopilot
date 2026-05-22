pub fn load_config() -> String {
    std::fs::read_to_string("config.json").unwrap()
}

pub fn future_work() {
    todo!("replace placeholder before release");
}
