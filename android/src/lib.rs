use jano::android_activity::{
    input::{InputEvent, KeyAction, KeyEvent, KeyMapChar, MotionAction},
    InputStatus,
};
use jano::android_activity::{AndroidApp, MainEvent};
use jano::{wgpu, FrameStats, TouchTranslater, Window};

use mlsim_common::app::{App, AppInput};
use mlsim_common::egui;
use mlsim_common::glam::{uvec2, vec2};
use mlsim_common::{save::Project, settings::Settings, Platform};

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

fn save_dir() -> PathBuf {
    jano::android().external_data_path().unwrap()
}

fn save_data<T: serde::Serialize>(
    filename: &str,
    data: &T,
) -> Result<PathBuf, (PathBuf, std::io::Error)> {
    let dir = save_dir();
    _ = std::fs::create_dir(&dir);
    let bytes = bincode::serialize(data).unwrap();
    let path = dir.join(filename);
    std::fs::write(&path, &bytes)
        .map(|()| path.clone())
        .map_err(|err| (path, err))
}

fn load_data<T: for<'a> serde::Deserialize<'a>>(filename: &str) -> std::io::Result<T> {
    let dir = save_dir();
    let bytes = std::fs::read(dir.join(filename))?;
    bincode::deserialize(&bytes).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to parse data from file {filename:?}"),
        )
    })
}

struct DisplayWindow(Window);
impl raw_window_handle::HasWindowHandle for DisplayWindow {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        raw_window_handle::HasWindowHandle::window_handle(&self.0)
    }
}
impl raw_window_handle::HasDisplayHandle for DisplayWindow {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(raw_window_handle::DisplayHandle::android())
    }
}

#[no_mangle]
fn android_main(android: AndroidApp) {
    android_logd_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("main", log::LevelFilter::Info)
        .init();
    jano::android_main(android, State::default(), 60);
}

static UI_SCALE: AtomicU32 = AtomicU32::new(100);

pub struct AndroidPlatform;
impl Platform for AndroidPlatform {
    fn set_scale_factor(scale: f32) {
        if scale >= 0.1 {
            UI_SCALE.store((scale * 100.0).round() as u32, Ordering::Relaxed)
        }
    }

    fn load_settings() -> std::io::Result<Settings> {
        log::info!("Reading settings.data...");
        load_data("settings.data")
    }
    fn save_settings(settings: Settings) -> std::io::Result<()> {
        let rs = save_data("settings.data", &settings);
        match &rs {
            Ok(path) => log::info!("Saved settings to {path:?}"),
            Err((path, err)) => log::warn!("Failed to save settings to {path:?} : {err:?}"),
        }
        rs.map(|_| ()).map_err(|(_path, err)| err)
    }

    #[rustfmt::skip]
    fn can_open_dirs() -> bool { false }

    #[allow(unreachable_code)]
    fn open_save_dir() -> std::io::Result<()> {
        panic!("Not supported")
    }

    fn list_available_projects() -> std::io::Result<Vec<String>> {
        let dir = save_dir();
        log::info!("Looking for project files in {dir:?}");
        let mut project_names: Vec<String> = Vec::new();

        for entry in std::fs::read_dir(&dir)?.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension() == Some(&std::ffi::OsString::from("project")) {
                let name = if let Some(os_str) = path.file_stem() {
                    os_str.to_string_lossy().to_string()
                } else {
                    String::from("")
                };
                project_names.push(name);
            }
        }
        Ok(project_names)
    }

    fn load_project(name: &str) -> std::io::Result<Project> {
        log::info!("Reading {name}.project...");
        load_data(&format!("{name}.project"))
    }
    fn save_project(name: &str, project: Project) -> std::io::Result<()> {
        let rs = save_data(&format!("{name}.project"), &project);
        match &rs {
            Ok(path) => log::info!("Saved project {name:?} to {path:?}"),
            Err((path, err)) => log::warn!("Failed to save project {name:?} to {path:?} : {err:?}"),
        }
        rs.map(|_| ()).map_err(|(_path, err)| err)
    }

    #[rustfmt::skip]
    fn can_pick_file() -> bool { true }

    async fn pick_file() -> std::io::Result<std::fs::File> {
        todo!()
    }
    async fn pick_files() -> std::io::Result<Vec<std::fs::File>> {
        todo!()
    }

    #[rustfmt::skip]
    fn has_external_data() -> bool { false }

    fn download_external_data() {
        panic!("Not supported")
    }
    fn upload_external_data() {
        panic!("Not supported")
    }

    #[rustfmt::skip]
    fn is_touchscreen() -> bool { true }
	#[rustfmt::skip]
    fn has_physical_keyboard() -> bool { false }
	#[rustfmt::skip]
    fn name() -> String { "Android".into() }
}

#[derive(Default)]
struct State {
    window: Option<Arc<DisplayWindow>>,
    app: App<AndroidPlatform>,
    input: egui::RawInput,
    translater: TouchTranslater,
    keyboard_showing: bool,
}
impl jano::AppState for State {
    fn on_main_event(&mut self, event: MainEvent, draw_frames: &mut bool) -> bool {
        match event {
            MainEvent::Pause => {
                _ = AndroidPlatform::save_settings(self.app.settings.clone());

                *draw_frames = false;
                self.app.invalidate_surface();
                log::info!("App paused");
            }
            MainEvent::Resume { .. } => {
                match AndroidPlatform::load_settings() {
                    Ok(settings) => self.app.settings = settings,
                    Err(err) => log::warn!("Failed to parse settings: {err:?}"),
                }

                *draw_frames = true;
                log::info!("App resumed");
            }
            MainEvent::InitWindow { .. } => {
                log::info!("App window initialized");
                self.window = jano::android()
                    .native_window()
                    .map(DisplayWindow)
                    .map(Arc::new);

                if let Some(win) = &self.window {
                    let instance = wgpu::Instance::new(Default::default());
                    let surface: wgpu::Surface<'static> =
                        instance.create_surface(win.clone()).unwrap();

                    let size = uvec2(win.0.width() as u32, win.0.height() as u32);
                    pollster::block_on(self.app.renew_surface(&instance, surface, size));
                } else {
                    log::error!("native_window() returned None during InitWindow callback");
                }
            }
            MainEvent::TerminateWindow { .. } => self.window = None,
            MainEvent::Destroy => return true,
            _ => {}
        }
        false
    }
    fn on_frame(&mut self, _stats: FrameStats) {
        // Handle input
        'i: {
            self.translater
                .update(|e| self.input.events.extend(Vec::<_>::from(e)));
            let mut iter = match jano::android().input_events_iter() {
                Ok(iter) => iter,
                Err(err) => {
                    log::warn!("Failed to get input events iterator: {err:?}");
                    break 'i;
                }
            };
            while iter.next(|event| handle_input_event(self, event)) {}
        }

        let Some(win) = &self.window else {
            return;
        };
        let win_size = uvec2(win.0.width() as u32, win.0.height() as u32);
        let cutouts = jano::display_cutout(vec2(win_size.x as f32, win_size.y as f32));
        let content_rect = egui::Rect::from_min_max(
            egui::pos2(cutouts.0.x, cutouts.0.y),
            egui::pos2(cutouts.1.x, cutouts.1.y),
        );

        let mut input = AppInput {
            egui_input: self.input.take(),
            fps: 0,
            content_rect,
            win_size,
        };

        // scaling
        {
            let input_scale = UI_SCALE.load(Ordering::Relaxed) as f32 * 0.01;
            let content_rect = {
                let (min, max) = (
                    content_rect.min / input_scale,
                    content_rect.max / input_scale,
                );
                egui::Rect::from_min_max(min, max)
            };
            let egui_input = &mut input.egui_input;
            let viewport = egui_input
                .viewports
                .get_mut(&egui::viewport::ViewportId::ROOT)
                .unwrap();
            viewport.native_pixels_per_point = Some(input_scale);
            viewport.inner_rect = Some(content_rect);
            egui_input.screen_rect = Some(content_rect);

            egui_input
                .events
                .iter_mut()
                .for_each(|event| *event = scale_event(event, input_scale));
        }

        match self.app.draw_frame(input) {
            Ok(platform_output) => {
                let show_keyboard = platform_output.ime.is_some();
                if show_keyboard != self.keyboard_showing {
                    self.keyboard_showing = show_keyboard;
                    _ = jano::set_keyboard_visibility(show_keyboard);
                }
            }
            Err(err) => log::warn!("Failed to draw frame: {err:?}"),
        }
    }
}

fn handle_input_event(state: &mut State, event: &InputEvent) -> InputStatus {
    match event {
        InputEvent::KeyEvent(key_event) => {
            let mut new_event = None;
            let combined_key_char =
                character_map_and_combine_key(key_event, &mut None, &mut new_event);
            match combined_key_char {
                Some(KeyMapChar::Unicode(ch)) | Some(KeyMapChar::CombiningAccent(ch)) => {
                    state.input.events.push(egui::Event::Text(ch.to_string()));
                }
                _ => {}
            }
            if let Some(event) = new_event {
                state.input.events.push(event);
            }
        }
        InputEvent::MotionEvent(motion_event) => {
            let idx = motion_event.pointer_index();
            let pointer = motion_event.pointer_at_index(idx);
            let pos = vec2(pointer.x(), pointer.y());
            let handler = |e: jano::TouchEvent| state.input.events.extend(Vec::<_>::from(e));
            let translater = &mut state.translater;

            match motion_event.action() {
                MotionAction::Down | MotionAction::PointerDown => {
                    translater.phase_start(idx, pos, handler)
                }
                MotionAction::Up | MotionAction::PointerUp | MotionAction::Cancel => {
                    translater.phase_end(idx, pos, handler)
                }
                MotionAction::Move => translater.phase_move(idx, pos, handler),
                a => log::warn!("Unknown motion action: {a:?}"),
            }
        }
        InputEvent::TextEvent(text_state) => {
            log::info!("Android set text input to {text_state:?}");
        }
        _ => return InputStatus::Unhandled,
    }
    InputStatus::Handled
}

/// Tries to map the `key_event` to a `KeyMapChar` containing a unicode character or dead key accent
fn character_map_and_combine_key(
    key_event: &KeyEvent,
    combining_accent: &mut Option<char>,
    out_event: &mut Option<egui::Event>,
) -> Option<KeyMapChar> {
    let device_id = key_event.device_id();

    log::info!(
        "Recieved KeyEvent {{ action: {:?}, key: {:?} }}",
        key_event.action(),
        key_event.key_code()
    );

    use jano::android_activity::input::Keycode;
    if key_event.key_code() == Keycode::Del {
        match key_event.action() {
            KeyAction::Up => {
                *out_event = Some(egui::Event::Key {
                    key: egui::Key::Backspace,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: Default::default(),
                })
            }
            KeyAction::Down => {
                *out_event = Some(egui::Event::Key {
                    key: egui::Key::Backspace,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                })
            }
            _ => {}
        }
        return None;
    }

    let key_map = match jano::android().device_key_character_map(device_id) {
        Ok(key_map) => key_map,
        Err(err) => {
            log::warn!("Failed to look up `KeyCharacterMap` for device {device_id}: {err:?}");
            return None;
        }
    };

    match key_map.get(key_event.key_code(), key_event.meta_state()) {
        Ok(KeyMapChar::Unicode(unicode)) => {
            // Only do dead key combining on key down
            if key_event.action() == KeyAction::Down {
                let combined_unicode = if let Some(accent) = combining_accent {
                    match key_map.get_dead_char(*accent, unicode) {
                        Ok(Some(key)) => {
                            log::warn!("KeyEvent: Combined '{unicode}' with accent '{accent}' to give '{key}'");
                            Some(key)
                        }
                        Ok(None) => None,
                        Err(err) => {
                            log::warn!("KeyEvent: Failed to combine 'dead key' accent '{accent}' with '{unicode}': {err:?}");
                            None
                        }
                    }
                } else {
                    Some(unicode)
                };
                *combining_accent = None;
                combined_unicode.map(KeyMapChar::Unicode)
            } else {
                // Some(KeyMapChar::Unicode(unicode))
                None
            }
        }
        Ok(KeyMapChar::CombiningAccent(accent)) => {
            if key_event.action() == KeyAction::Down {
                *combining_accent = Some(accent);
            }
            Some(KeyMapChar::CombiningAccent(accent))
        }
        Ok(KeyMapChar::None) => {
            // Leave any combining_accent state in tact (seems to match how other
            // Android apps work)
            log::warn!("KeyEvent: Pressed non-unicode key");
            None
        }
        Err(err) => {
            log::warn!("KeyEvent: Failed to get key map character: {err:?}");
            *combining_accent = None;
            None
        }
    }
}

fn scale_event(e: &egui::Event, scale: f32) -> egui::Event {
    use egui::Event::*;
    match e {
        PointerMoved(pos) => PointerMoved(*pos / scale),
        MouseMoved(delta) => MouseMoved(*delta / scale),
        PointerButton {
            pos,
            button,
            pressed,
            modifiers,
        } => PointerButton {
            pos: *pos / scale,
            button: *button,
            pressed: *pressed,
            modifiers: *modifiers,
        },
        e => e.clone(),
    }
}
