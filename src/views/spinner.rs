use std::time::Instant;

/// Braille spinner frames for smooth animation
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Get the current spinner frame based on elapsed time
pub fn spinner_frame(start: Option<Instant>) -> &'static str {
    let start = match start {
        Some(s) => s,
        None => return SPINNER_FRAMES[0],
    };

    let elapsed = start.elapsed().as_millis();
    // Change frame every 80ms for smooth animation
    let frame_index = (elapsed / 80) as usize % SPINNER_FRAMES.len();
    SPINNER_FRAMES[frame_index]
}
