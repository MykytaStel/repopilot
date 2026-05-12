use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::time::Duration;

pub fn make_spinner(message: &'static str) -> Option<ProgressBar> {
    if !std::io::stderr().is_terminal() {
        return None;
    }

    let style = ProgressStyle::with_template("{spinner:.cyan} {msg}")
        .ok()?
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);

    let pb = ProgressBar::new_spinner();
    pb.set_style(style);
    pb.set_message(message);
    pb.enable_steady_tick(Duration::from_millis(80));
    Some(pb)
}

pub fn finish_spinner(pb: Option<ProgressBar>) {
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }
}
