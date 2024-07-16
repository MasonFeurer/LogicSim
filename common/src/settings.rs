use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum UiTheme {
    Light,
    Dark,
    Night,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub ui_scale: f32,
    pub ui_theme: UiTheme,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            ui_theme: UiTheme::Dark,
        }
    }
}
