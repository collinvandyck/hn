use super::ThemeVariant;

/// Detect terminal background color mode
pub fn detect_terminal_theme() -> ThemeVariant {
    // Use terminal-light crate for detection
    // Falls back to dark if detection fails
    match terminal_light::luma() {
        Ok(luma) if luma > 0.5 => ThemeVariant::Light,
        Ok(_) => ThemeVariant::Dark,
        Err(_) => {
            // Detection failed, default to dark (most common for terminals)
            ThemeVariant::Dark
        }
    }
}
