use super::ThemeVariant;

pub fn detect_terminal_theme() -> ThemeVariant {
    match terminal_light::luma() {
        Ok(luma) if luma > 0.5 => ThemeVariant::Light,
        Ok(_) => ThemeVariant::Dark,
        Err(_) => ThemeVariant::Dark,
    }
}
