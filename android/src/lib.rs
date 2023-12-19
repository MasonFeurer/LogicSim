use logisim_common as logisim;

use logisim::app::App;
use logisim::glam::{uvec2, vec2, UVec2, Vec2};
use logisim::graphics::Rect;
use logisim::input::{InputEvent as LogisimInputEvent, InputState, PtrButton, TextInputState};

use android_activity::{
    input::{InputEvent, KeyAction, KeyEvent, KeyMapChar, MotionAction},
    AndroidApp, InputStatus, MainEvent, PollEvent,
};
use ndk::native_window::NativeWindow;
use raw_window_handle::{
    AndroidDisplayHandle, HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle,
    RawWindowHandle,
};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, SystemTime};

static TOP_DISPLAY_INSET: AtomicI32 = AtomicI32::new(0);
static RIGHT_DISPLAY_INSET: AtomicI32 = AtomicI32::new(0);
static BOTTOM_DISPLAY_INSET: AtomicI32 = AtomicI32::new(0);
static LEFT_DISPLAY_INSET: AtomicI32 = AtomicI32::new(0);

#[allow(dead_code)]
#[allow(non_snake_case)]
#[no_mangle]
/// Callback from Java code to update display insets (cutouts).
pub extern "C" fn Java_com_logisim_android_MainActivity_onDisplayInsets(
    env: jni::JNIEnv,
    _class: jni::objects::JObject,
    cutouts: jni::sys::jarray,
) {
    use jni::objects::{JObject, JPrimitiveArray};

    let mut array: [i32; 4] = [0; 4];
    unsafe {
        let j_obj = JObject::from_raw(cutouts);
        let j_arr = JPrimitiveArray::from(j_obj);
        env.get_int_array_region(j_arr, 0, array.as_mut()).unwrap();
    }

    TOP_DISPLAY_INSET.store(array[0], Ordering::Relaxed);
    RIGHT_DISPLAY_INSET.store(array[1], Ordering::Relaxed);
    BOTTOM_DISPLAY_INSET.store(array[2], Ordering::Relaxed);
    LEFT_DISPLAY_INSET.store(array[3], Ordering::Relaxed);
    log::info!("Settings DISPLAY_INSETS to {array:?}");
}

#[derive(Clone)]
struct Window {
    inner: NativeWindow,
}
impl Window {
    fn new(inner: NativeWindow) -> Self {
        Self { inner }
    }

    fn size(&self) -> UVec2 {
        uvec2(self.inner.width() as u32, self.inner.height() as u32)
    }
}
unsafe impl HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> RawWindowHandle {
        HasRawWindowHandle::raw_window_handle(&self.inner)
    }
}
unsafe impl HasRawDisplayHandle for Window {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Android(AndroidDisplayHandle::empty())
    }
}

#[derive(Debug, Clone, Copy)]
struct Ptr {
    pos: Vec2,
}

#[derive(Debug)]
struct Zoom {
    start_dist: f32,
    prev_dist: f32,
    anchor: Vec2,
}

#[derive(Debug)]
struct TouchTranslater {
    ignore_release: bool,
    last_press_time: SystemTime,
    last_pos: Vec2,
    press_pos: Option<Vec2>,
    holding: bool,
    pointer_count: u32,
    pointers: Vec<Option<Ptr>>,
    zoom: Option<Zoom>,
}
impl Default for TouchTranslater {
    fn default() -> Self {
        Self {
            ignore_release: false,
            last_press_time: SystemTime::UNIX_EPOCH,
            last_pos: Vec2::ZERO,
            press_pos: None,
            holding: false,
            pointer_count: 0,
            pointers: vec![],
            zoom: None,
        }
    }
}
impl TouchTranslater {
    fn update(&mut self, mut out: impl FnMut(LogisimInputEvent)) {
        if self.holding
            && SystemTime::now()
                .duration_since(self.last_press_time)
                .unwrap_or(Duration::ZERO)
                .as_millis()
                > 500
        {
            out(LogisimInputEvent::Click(self.last_pos, PtrButton::RIGHT));
            self.ignore_release = true;
            self.holding = false;
        }
    }

    fn phase_start(&mut self, idx: usize, pos: Vec2, mut out: impl FnMut(LogisimInputEvent)) {
        log::info!("Started motion: {idx} {pos:?}");

        self.pointer_count += 1;
        self.pointers.resize(idx + 1, None);
        self.pointers[idx] = Some(Ptr { pos });

        if self.pointer_count == 2 {
            self.press_pos = None;
            self.ignore_release = true;
            self.holding = false;

            out(LogisimInputEvent::PointerLeft);
            out(LogisimInputEvent::Release(PtrButton::LEFT));

            let mut pointers = self.pointers.iter().cloned().filter_map(|ptr| ptr);
            let [a, b] = [pointers.next().unwrap(), pointers.next().unwrap()];
            let dist = a.pos.distance_squared(b.pos);
            let anchor = Rect::from_min_max(a.pos.min(b.pos), a.pos.max(b.pos)).center();
            self.zoom = Some(Zoom {
                start_dist: dist,
                prev_dist: dist,
                anchor,
            });
        } else {
            out(LogisimInputEvent::Hover(pos));
            out(LogisimInputEvent::Press(pos, PtrButton::LEFT));

            self.last_pos = pos;
            self.last_press_time = SystemTime::now();
            self.press_pos = Some(pos);
            self.holding = true;
            self.ignore_release = false;
        }
    }

    fn phase_move(&mut self, idx: usize, pos: Vec2, mut out: impl FnMut(LogisimInputEvent)) {
        self.last_pos = pos;
        if self.pointer_count == 1 {
            out(LogisimInputEvent::Hover(pos));
        }

        if let Some(press_pos) = self.press_pos {
            let press_dist = press_pos.distance_squared(pos).abs();
            if press_dist >= 50.0 * 50.0 {
                self.holding = false;
                self.press_pos = None;
            }
        }
        if let Some(ptr) = self.pointers.get_mut(idx).unwrap() {
            ptr.pos = pos;
        }
        if self.pointer_count == 2 {
            let mut pointers = self.pointers.iter().cloned().filter_map(|ptr| ptr);
            let [a, b] = [pointers.next().unwrap(), pointers.next().unwrap()];
            let dist = a.pos.distance_squared(b.pos);
            let zoom = self.zoom.as_ref().unwrap();
            if dist != zoom.start_dist {
                let delta = (dist - zoom.prev_dist) * 0.0003;
                out(LogisimInputEvent::Zoom(zoom.anchor, delta));
            }

            self.zoom.as_mut().unwrap().prev_dist = dist;
        }
    }

    fn phase_end(&mut self, idx: usize, pos: Vec2, mut out: impl FnMut(LogisimInputEvent)) {
        out(LogisimInputEvent::Release(PtrButton::LEFT));

        // If we've been holding the pointer still and have not
        // triggered a right click, we should send a left click
        if !self.ignore_release && self.holding {
            out(LogisimInputEvent::Click(pos, PtrButton::LEFT));
        }
        self.press_pos = None;
        self.holding = false;
        out(LogisimInputEvent::PointerLeft);

        if self.pointer_count == 2 {
            self.zoom = None;
        }

        self.pointers[idx] = None;
        self.pointer_count -= 1;
    }
}

struct State {
    combining_accent: Option<char>,
    window: Option<Window>,
    quit: bool,
    running: bool,
    app: App,
    android: AndroidApp,
    input: InputState,
    translater: TouchTranslater,
    text_input: Option<TextInputState>,

    frame_count: u32,
    last_fps_update: SystemTime,
    fps: u32,
}

#[no_mangle]
fn android_main(android: AndroidApp) {
    android_logd_logger::builder()
        .filter_level(log::LevelFilter::Error)
        .filter_module("logisim_common", log::LevelFilter::Info)
        .filter_module("main", log::LevelFilter::Info)
        .init();

    let mut state = State {
        combining_accent: None,
        window: None,
        quit: false,
        running: false,
        app: App::new(),
        android: android.clone(),
        input: InputState::default(),
        translater: TouchTranslater::default(),
        text_input: None,

        frame_count: 0,
        last_fps_update: SystemTime::now(),
        fps: 0,
    };
    let mut last_frame_time = SystemTime::now();
    let timeout = Duration::from_millis(1000 / 60);

    while !state.quit {
        android.poll_events(Some(timeout), |event| {
            match event {
                PollEvent::Wake => {}
                PollEvent::Timeout => {}
                PollEvent::Main(main_event) => {
                    handle_main_event(main_event, &mut state);
                }
                _ => {}
            }

            let redraw = SystemTime::now()
                .duration_since(last_frame_time)
                .unwrap_or(Duration::ZERO)
                .as_millis()
                > (1000 / 60);
            if redraw && state.running {
                // Update FPS
                {
                    state.frame_count += 1;
                    if SystemTime::now()
                        .duration_since(state.last_fps_update)
                        .unwrap()
                        .as_secs()
                        >= 1
                    {
                        state.last_fps_update = SystemTime::now();
                        state.fps = state.frame_count;
                        state.frame_count = 0;
                    }
                }

                last_frame_time = SystemTime::now();
                draw_frame(&mut state);
            }
        });
    }
}

fn handle_main_event(event: MainEvent, state: &mut State) {
    match event {
        MainEvent::SaveState { .. } => {}
        MainEvent::Pause => {
            log::info!("App paused - dropping display & GPU handles...");

            state.running = false;
            state.app.pause();
        }
        MainEvent::Resume { .. } => {
            log::info!("App resumed");
            state.running = true;
        }
        MainEvent::InitWindow { .. } => {
            log::info!("Window initialized - creating display & GPU handles...");
            state.window = state.android.native_window().map(Window::new);

            if let Some(win) = &state.window {
                pollster::block_on(state.app.resume(win, win.size()));
            } else {
                log::error!("native_window() returned None during InitWindow callback");
            }
        }
        MainEvent::TerminateWindow { .. } => {
            log::info!("App terminated...");
            state.window = None;
        }
        MainEvent::WindowResized { .. } => {
            if let Some(win) = &state.window {
                log::info!("Resizing app to {:?}", win.size());
                state.app.update_size(win.size());
            } else {
                log::error!("Handling WindowResized but window is None");
            }
        }
        MainEvent::RedrawNeeded { .. } => {}
        MainEvent::InputAvailable { .. } => {}
        MainEvent::ConfigChanged { .. } => {}
        MainEvent::LowMemory => log::warn!("Recieved LowMemory Event..."),
        MainEvent::Destroy => {
            log::info!("App destroyed...");
            state.quit = true;
        }
        _ => {}
    }
}

fn handle_input_event(state: &mut State, event: &InputEvent) -> InputStatus {
    let out = &mut state.input;
    match event {
        InputEvent::KeyEvent(key_event) => {
            let combined_key_char = character_map_and_combine_key(
                &state.android,
                key_event,
                &mut state.combining_accent,
            );
            match combined_key_char {
                Some(KeyMapChar::Unicode(ch)) | Some(KeyMapChar::CombiningAccent(ch)) => {
                    out.on_event(LogisimInputEvent::Type(ch));
                }
                _ => {}
            }
        }
        InputEvent::MotionEvent(motion_event) => {
            let idx = motion_event.pointer_index();
            let pointer = motion_event.pointer_at_index(idx);
            let pos = vec2(pointer.x(), pointer.y());
            let handler = |e: LogisimInputEvent| out.on_event(e);
            let translater = &mut state.translater;

            match motion_event.action() {
                MotionAction::Down | MotionAction::PointerDown => {
                    log::info!("Pressed pointer; idx: {idx}");
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
            let min = text_state.selection.start.min(text_state.selection.end) as u32;
            let max = text_state.selection.start.max(text_state.selection.end) as u32;
            let compose = text_state.compose_region.and_then(|span| {
                let min = span.start.min(span.end);
                let max = span.start.max(span.end);
                match min == max {
                    true => None,
                    false => Some(min as u32..max as u32),
                }
            });

            let mut info = TextInputState {
                text: text_state.text.clone(),
                selection: min..max,
                compose,
            };
            // Temporary fix for backspace deleting 2 characters on android keyboard.
            if let Some(input) = &state.text_input {
                if input.text.len().saturating_sub(info.text.len()) == 2 {
                    info.text.push(input.text.chars().rev().nth(1).unwrap());
                    info.selection.end += 1;
                    if let Some(range) = &mut info.compose {
                        range.end += 1;
                    }
                }

                state
                    .android
                    .set_text_input_state(android_activity::input::TextInputState {
                        text: info.text.clone(),
                        selection: android_activity::input::TextSpan {
                            start: info.selection.start as usize,
                            end: info.selection.end as usize,
                        },
                        compose_region: info.compose.as_ref().map(|range| {
                            android_activity::input::TextSpan {
                                start: range.start as usize,
                                end: range.end as usize,
                            }
                        }),
                    });
            }
            log::info!("Android set TextInput to {info:?}");
            out.set_text_input(Some(info.clone()));
            state.text_input = Some(info);
        }
        _ => return InputStatus::Unhandled,
    }
    InputStatus::Handled
}

fn draw_frame(state: &mut State) {
    // Handle input
    'i: {
        state.translater.update(|e| state.input.on_event(e));
        let android = state.android.clone();
        let mut iter = match android.input_events_iter() {
            Ok(iter) => iter,
            Err(err) => {
                log::warn!("Failed to get input events iterator: {err:?}");
                break 'i;
            }
        };
        while iter.next(|event| handle_input_event(state, event)) {}
    }

    let Some(_win) = &state.window else {
        log::warn!("Failed to draw frame: window is None");
        return;
    };

    // Determine screen area
    let content_rect = state.android.content_rect();
    let mut content_rect = logisim_common::graphics::Rect::from_min_max(
        logisim_common::glam::vec2(content_rect.left as f32, content_rect.top as f32),
        logisim_common::glam::vec2(content_rect.right as f32, content_rect.bottom as f32),
    );
    content_rect.min += logisim_common::glam::vec2(
        LEFT_DISPLAY_INSET.load(Ordering::Relaxed) as f32,
        TOP_DISPLAY_INSET.load(Ordering::Relaxed) as f32,
    );
    content_rect.max -= logisim_common::glam::vec2(
        RIGHT_DISPLAY_INSET.load(Ordering::Relaxed) as f32,
        BOTTOM_DISPLAY_INSET.load(Ordering::Relaxed) as f32,
    );

    // Draw frame
    let mut text_input = None;
    match state
        .app
        .draw_frame(&mut state.input, content_rect, &mut text_input, state.fps)
    {
        Err(err) => {
            log::warn!("Failed to draw frame: {err:?}");
            return;
        }
        Ok(_) => {}
    }

    // Handle text input
    state.input.update();
    if state.text_input.is_none() && text_input.is_some() {
        log::info!("App started wanting text input ;' opening keyboard");
        state.android.show_soft_input(true);
    }
    if state.text_input.is_some() && text_input.is_none() {
        log::info!("App stopped wanting text input ;' closing keyboard");
        state.android.hide_soft_input(true);
    }

    if text_input.is_some() && text_input != state.text_input {
        let text = text_input.as_ref().expect("Can't happen");
        log::info!("Setting androids TextInput to {text_input:?}");
        state
            .android
            .set_text_input_state(android_activity::input::TextInputState {
                text: text.text.clone(),
                selection: android_activity::input::TextSpan {
                    start: text.selection.start as usize,
                    end: text.selection.end as usize,
                },
                compose_region: text.compose.as_ref().map(|range| {
                    android_activity::input::TextSpan {
                        start: range.start as usize,
                        end: range.end as usize,
                    }
                }),
            });
    }
    state.text_input = text_input;
}

/// Tries to map the `key_event` to a `KeyMapChar` containing a unicode character or dead key accent
///
/// This shows how to take a `KeyEvent` and look up its corresponding `KeyCharacterMap` and
/// use that to try and map the `key_code` + `meta_state` to a unicode character or a
/// dead key that be combined with the next key press.
fn character_map_and_combine_key(
    android: &AndroidApp,
    key_event: &KeyEvent,
    combining_accent: &mut Option<char>,
) -> Option<KeyMapChar> {
    let device_id = key_event.device_id();

    let key_map = match android.device_key_character_map(device_id) {
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
                combined_unicode.map(|unicode| KeyMapChar::Unicode(unicode))
            } else {
                Some(KeyMapChar::Unicode(unicode))
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
