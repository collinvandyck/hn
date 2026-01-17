use std::time::Instant;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spinner_frame(start: Option<Instant>) -> &'static str {
    let start = match start {
        Some(s) => s,
        None => return SPINNER_FRAMES[0],
    };

    let elapsed = start.elapsed().as_millis();
    let frame_index = (elapsed / 80) as usize % SPINNER_FRAMES.len();
    SPINNER_FRAMES[frame_index]
}
