use std::{env, fs};

fn main() {
    let path = env::args().nth(1).expect("report path");
    let content = fs::read_to_string(path).expect("read report");
    println!(
        "0.20.0 {}",
        outcome(repopilot020::api::report::parse_scan_summary_json(&content))
    );
    println!(
        "0.21.0 {}",
        outcome(repopilot021::api::report::parse_scan_summary_json(&content))
    );
    println!(
        "0.22.0 {}",
        outcome(repopilot022::api::report::parse_scan_summary_json(&content))
    );
}

fn outcome<T, E: std::fmt::Display>(result: Result<T, E>) -> String {
    match result {
        Ok(_) => "ACCEPT".to_string(),
        Err(error) => format!("REJECT {error}"),
    }
}
