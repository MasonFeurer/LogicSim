use logisim_common as logisim;

use logisim::app::{App, FrameOutput};
use logisim::glam::{uvec2, vec2, UVec2, Vec2};
use logisim::input::{InputEvent, InputState, PtrButton};
use logisim::save::Library;
use logisim::{log, Rect};

use web_time::{Duration, SystemTime};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys;

use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;

pub fn get_os() -> Option<&'static str> {
    let mut os_name = "Unknown";
    let navigator = web_sys::window()?.navigator();
    let app_version = navigator.app_version().ok()?;

    if app_version.contains("Win") {
        os_name = "windows";
    }
    if app_version.contains("Mac") {
        os_name = "macos";
    }
    if app_version.contains("X11") {
        os_name = "unix";
    }
    if app_version.contains("Linux") {
        os_name = "linux";
    }
    Some(os_name)
}

type MergeLibraries = (Arc<SyncSender<Library>>, Receiver<Library>);
static mut MERGE_LIBRARIES: Option<MergeLibraries> = None;
fn merge_libraries() -> &'static MergeLibraries {
    unsafe { MERGE_LIBRARIES.as_ref().unwrap() }
}

struct State {
    app: App,
    window: Window,
    input: InputState,
    last_frame_time: SystemTime,
    last_size: UVec2,

    frame_count: u32,
    last_fps_update: SystemTime,
    fps: u32,
}

fn canvas_by_id(canvas_id: &str) -> Option<web_sys::HtmlCanvasElement> {
    use wasm_bindgen::JsCast as _;
    let document = web_sys::window()?.document()?;
    let canvas = document.get_element_by_id(canvas_id)?;
    canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok()
}

fn native_pixels_per_point() -> f32 {
    let pixels_per_point = web_sys::window().unwrap().device_pixel_ratio() as f32;
    if pixels_per_point > 0.0 && pixels_per_point.is_finite() {
        pixels_per_point
    } else {
        1.0
    }
}

fn screen_size(canvas: &web_sys::HtmlCanvasElement) -> UVec2 {
    let Some(parent) = canvas.parent_element() else {
        log::error!("Canvas somehow doesn't have a parent element!");
        return UVec2::ZERO;
    };
    let width = parent.scroll_width();
    let height = parent.scroll_height();

    if width <= 0 || height <= 0 {
        log::error!("Canvas parent size is {width}x{height}. Try adding `html, body {{ height: 100%; width: 100% }}` to your CSS!");
    }
    uvec2(width as u32, height as u32)
}

fn resize_canvas(canvas: &web_sys::HtmlCanvasElement, size: UVec2) -> Option<()> {
    let size_pixels = native_pixels_per_point() * size.as_vec2();

    // Make sure that the height and width are always even numbers.
    // otherwise, the page renders blurry on some platforms.
    // See https://github.com/emilk/egui/issues/103
    fn round_to_even(v: f32) -> f32 {
        (v / 2.0).round() * 2.0
    }

    canvas
        .style()
        .set_property("width", &format!("{}px", round_to_even(size.x as f32)))
        .ok()?;
    canvas
        .style()
        .set_property("height", &format!("{}px", round_to_even(size.y as f32)))
        .ok()?;
    canvas.set_width(round_to_even(size_pixels.x) as u32);
    canvas.set_height(round_to_even(size_pixels.y) as u32);

    Some(())
}

use std::sync::atomic::{AtomicBool, Ordering};

static TRIGGERED_SAVE: AtomicBool = AtomicBool::new(false);

#[wasm_bindgen]
// Should be called by site script when the tab is about to be closed.
pub async fn trigger_save() {
    TRIGGERED_SAVE.store(true, Ordering::SeqCst);
}

#[wasm_bindgen]
pub async fn main_web(canvas_id: &str) {
    unsafe {
        let (sender, receiver) = sync_channel(1000);
        MERGE_LIBRARIES = Some((Arc::new(sender), receiver));
    }

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    _ = console_log::init();

    use winit::platform::web::EventLoopExtWebSys as _;
    use winit::platform::web::WindowBuilderExtWebSys as _;

    let Some(canvas) = canvas_by_id(canvas_id) else {
        panic!("Canvas with ID {canvas_id} does not exist");
    };
    log::info!("Using Canvas with ID {canvas_id:?}...");
    resize_canvas(&canvas, screen_size(&canvas));

    let size = uvec2(canvas.width(), canvas.height());
    let event_loop = EventLoopBuilder::new().build().unwrap();
    let window = winit::window::WindowBuilder::new()
        .with_canvas(Some(canvas))
        .build(&event_loop)
        .unwrap();

    let mut state = State {
        app: App::new(),
        input: InputState::default(),
        last_frame_time: SystemTime::now(),
        last_size: size,
        window,

        frame_count: 0,
        last_fps_update: SystemTime::now(),
        fps: 0,
    };
    state.app.external_data = true;

    if let Some(data) = load_data("library") {
        match bincode::deserialize(&data) {
            Ok(library) => state.app.library = library,
            Err(err) => log::warn!("Failed to parse library data in localStorage: {err:?}"),
        }
    }
    if let Some(data) = load_data("scenes") {
        match bincode::deserialize(&data) {
            Ok(scenes) => state.app.scenes = scenes,
            Err(err) => log::warn!("Failed to parse scenes data in localStorage: {err:?}"),
        }
    }
    if let Some(data) = load_data("settings") {
        match bincode::deserialize(&data) {
            Ok(settings) => state.app.settings = settings,
            Err(err) => log::warn!("Failed to parse settings data in localStorage: {err:?}"),
        }
    }

    log::info!("Starting app with size {size:?}");
    state.app.resume(&state.window, size).await;
    state.app.update_size(size);
    state.window.request_redraw();

    event_loop.spawn(move |event, elwt| {
        // merge imported libraries
        if let Ok(lib2) = merge_libraries().1.try_recv() {
            state.app.library.tables.extend(lib2.tables);
            state
                .app
                .library
                .chips
                .extend(lib2.chips.into_iter().filter(|chip| !chip.builtin));
        }

        let mut exit = false;
        on_event(&mut state, event, &mut exit);
        if exit {
            elwt.exit();
        } else {
            if TRIGGERED_SAVE
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                log::info!("Saving app...");
                let data = bincode::serialize(&state.app.library).unwrap();
                save_data(&data, "library");
                let data = bincode::serialize(&state.app.scenes).unwrap();
                save_data(&data, "scenes");
                let data = bincode::serialize(&state.app.settings).unwrap();
                save_data(&data, "settings");
            }

            elwt.set_control_flow(ControlFlow::Wait);
        }
    });
}

/// Saves some data in the browsers `localStorage` with some key.
fn save_data(data: &[u8], tag: &str) {
    // The data stored in localStorage must be Strings.
    // And this string must be valid UTF-8 (I tried constructing an illegal
    // string with std::str::from_utf8_unchecked, but it was caught by the JS bindings).
    // Converting the binary data to a string with the Display formatter,
    // for example, would be very inefficient.
    // So here, I make the array twice as large by splitting each byte into 2 4-bit integers,
    // making a String that is guarenteed to be valid UTF-8.
    // For example: [0b11010011, 0b0001001] (not valid ASCII, probably not valid UTF-8), gets
    // converted into: [0b0011, 0b1101, 0b1001, 0b0001] (valid ASCII, thus valid UTF-8).
    let mut data_wide = Vec::with_capacity(data.len() * 2);
    for b in data {
        // [LSW, MSW]
        data_wide.push(*b & 0xF);
        data_wide.push((*b & 0xF0) >> 4);
    }
    let data_str = unsafe { std::str::from_utf8_unchecked(&data_wide) };
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    storage.set(tag, data_str).unwrap();
}

/// Loads some data from the browsers `localStorage` with some key.
fn load_data(tag: &str) -> Option<Vec<u8>> {
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    let bytes = storage.get(tag).unwrap().map(String::into_bytes)?;
    assert!(bytes.len() % 2 == 0);
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for idx in 0..bytes.len() / 2 {
        out.push(bytes[idx * 2] | (bytes[idx * 2 + 1] << 4));
    }
    Some(out)
}

fn on_event(state: &mut State, event: Event<()>, exit: &mut bool) {
    match event {
        Event::Resumed => log::info!("Received Resumed Event"),
        Event::Suspended => log::info!("Received Suspended Event"),
        Event::WindowEvent { event, .. } => on_window_event(state, event, exit),
        Event::LoopExiting => log::info!("Received LoopExiting Event"),
        _ => {}
    }
}

/// Downloads a chunk of binary data as a file with the name `filename`.
fn download_data(data: &[u8], filename: &str) -> Result<(), wasm_bindgen::JsValue> {
    // -- Create download URL --
    // Safety: the u8_arr is valid as long as no new memory is allocated
    let u8_arr = unsafe { js_sys::Uint8Array::view(&data) };

    let seq = js_sys::Array::new_with_length(1);
    seq.set(0, u8_arr.into());

    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
        &seq,
        web_sys::BlobPropertyBag::new().type_("application/octet-stream"),
    )?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    log::info!("Download data at URL {url:?} (should be invalid after download starts!)");

    // -- Place download anchor in page --
    let document = web_sys::window().unwrap().document().unwrap();
    let anchor = document.create_element("a")?;
    anchor.set_attribute("href", &url)?;
    anchor.set_attribute("download", filename)?;
    document.body().unwrap().append_child(&anchor)?;

    // -- Click download anchor, triggering browser to download data --
    anchor.unchecked_ref::<web_sys::HtmlElement>().click();

    // -- Clean up --
    document.body().unwrap().remove_child(&anchor)?;
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

fn on_window_event(ctx: &mut State, event: WindowEvent, exit: &mut bool) {
    match event {
        WindowEvent::RedrawRequested => {
            use winit::platform::web::WindowExtWebSys as _;

            let redraw = SystemTime::now()
                .duration_since(ctx.last_frame_time)
                .unwrap_or(Duration::ZERO)
                .as_millis()
                > (1000 / 60);

            if redraw {
                // Update FPS
                {
                    ctx.frame_count += 1;
                    if SystemTime::now()
                        .duration_since(ctx.last_fps_update)
                        .unwrap()
                        .as_secs()
                        >= 1
                    {
                        ctx.last_fps_update = SystemTime::now();
                        ctx.fps = ctx.frame_count;
                        ctx.frame_count = 0;
                    }
                }

                let canvas = &ctx.window.canvas().unwrap();
                let screen_size = screen_size(canvas);
                if ctx.last_size != screen_size {
                    ctx.last_size = screen_size;
                    resize_canvas(canvas, screen_size);
                    ctx.app.update_size(screen_size);
                    log::info!("Resizing app to {screen_size:?}");
                }

                let content_rect = Rect::from_min_size(Vec2::ZERO, screen_size.as_vec2());

                ctx.last_frame_time = SystemTime::now();
                let mut out = FrameOutput::default();
                if let Err(err) = ctx.app.draw_frame(
                    &mut ctx.input,
                    content_rect,
                    &mut Default::default(),
                    ctx.fps,
                    &mut out,
                ) {
                    log::warn!("Failed to draw frame: {err:?}");
                }
                if out.download_data {
                    log::info!("Downloading Library data...");
                    let bytes = bincode::serialize(&ctx.app.library).unwrap();
                    if let Err(err) = download_data(&bytes, "library.data") {
                        log::error!("Error downloading library: {err:?}");
                    }
                }
                if out.import_data {
                    let sender = std::sync::Arc::clone(&merge_libraries().0);
                    let future = async move {
                        let entries = rfd::AsyncFileDialog::new().pick_files().await;
                        for entry in entries.unwrap_or(Vec::new()) {
                            let bytes = entry.read().await;
                            let Ok(library) = bincode::deserialize::<Library>(&bytes) else {
                                log::error!("failed to parse library {:?}", entry.file_name());
                                continue;
                            };
                            sender.send(library).unwrap();
                        }
                    };
                    wasm_bindgen_futures::spawn_local(future);
                }
                ctx.input.update();
                ctx.window.request_redraw();
            }
            ctx.window.request_redraw();
        }
        WindowEvent::Resized(_size) => {}
        WindowEvent::CloseRequested => *exit = true,
        WindowEvent::CursorMoved { position, .. } => {
            let pos: [f32; 2] = position.into();
            ctx.input.on_event(InputEvent::Hover(pos.into()));
        }
        WindowEvent::MouseInput { state, button, .. } => {
            let button = match button {
                MouseButton::Left => PtrButton::LEFT,
                MouseButton::Middle => PtrButton::MIDDLE,
                MouseButton::Right => PtrButton::RIGHT,
                MouseButton::Back => PtrButton::BACK,
                MouseButton::Forward => PtrButton::FORWARD,
                MouseButton::Other(id) => PtrButton::new(id),
            };
            let pos = ctx.input.ptr_pos();
            if state == ElementState::Pressed {
                ctx.input.on_event(InputEvent::Click(pos, button));
                ctx.input.on_event(InputEvent::Press(pos, button));
            } else {
                ctx.input.on_event(InputEvent::Release(button));
            }
        }
        WindowEvent::MouseWheel { delta, .. } => match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                ctx.input.on_event(InputEvent::Scroll(vec2(x, y) * 0.01))
            }
            MouseScrollDelta::PixelDelta(pos) => ctx
                .input
                .on_event(InputEvent::Scroll(vec2(pos.x as f32, pos.y as f32) * 0.01)),
        },
        WindowEvent::TouchpadMagnify { delta, .. } => {
            if !ctx.input.ptr_gone() {
                ctx.input
                    .on_event(InputEvent::Zoom(ctx.input.ptr_pos(), delta as f32))
            }
        }
        WindowEvent::KeyboardInput { event, .. } => {
            if matches!(event.state, ElementState::Pressed) {
                match event.logical_key {
                    Key::Named(key) => {
                        if let Some(key) = translate_key(key) {
                            ctx.input.on_event(InputEvent::PressKey(key));
                        }
                    }
                    Key::Character(ref smol_str) => {
                        if smol_str.as_str() == "v" && ctx.input.modifiers().cmd {
                            // Paste command
                            return;
                        }
                        if smol_str.as_str() == "c" && ctx.input.modifiers().cmd {
                            // Copy command (For now we copy the entire active text field)
                            return;
                        }
                        for ch in smol_str.as_str().chars() {
                            ctx.input.on_event(InputEvent::Type(ch))
                        }
                    }
                    _ => {}
                }
            }
            if matches!(event.state, ElementState::Released) {
                if let Key::Named(key) = event.logical_key {
                    if let Some(key) = translate_key(key) {
                        ctx.input.on_event(InputEvent::ReleaseKey(key));
                    }
                }
            }
        }
        _ => {}
    }
}

fn translate_key(key: NamedKey) -> Option<logisim::input::Key> {
    Some(match key {
        NamedKey::Shift => logisim::input::Key::Shift,
        NamedKey::Control => logisim::input::Key::Command,
        NamedKey::Alt => logisim::input::Key::Option,

        NamedKey::Backspace => logisim::input::Key::Backspace,
        NamedKey::Enter => logisim::input::Key::Enter,
        NamedKey::Escape => logisim::input::Key::Esc,
        NamedKey::ArrowLeft => logisim::input::Key::Left,
        NamedKey::ArrowRight => logisim::input::Key::Right,
        NamedKey::ArrowUp => logisim::input::Key::Up,
        NamedKey::ArrowDown => logisim::input::Key::Down,
        NamedKey::Tab => logisim::input::Key::Tab,
        NamedKey::Space => logisim::input::Key::Space,
        NamedKey::Delete => logisim::input::Key::Delete,
        NamedKey::Insert => logisim::input::Key::Insert,
        NamedKey::Home => logisim::input::Key::Home,
        NamedKey::End => logisim::input::Key::End,
        NamedKey::PageUp => logisim::input::Key::PageUp,
        NamedKey::PageDown => logisim::input::Key::PageDown,
        NamedKey::F1 => logisim::input::Key::F1,
        NamedKey::F2 => logisim::input::Key::F2,
        NamedKey::F3 => logisim::input::Key::F3,
        NamedKey::F4 => logisim::input::Key::F4,
        NamedKey::F5 => logisim::input::Key::F5,
        NamedKey::F6 => logisim::input::Key::F6,
        NamedKey::F7 => logisim::input::Key::F7,
        NamedKey::F8 => logisim::input::Key::F8,
        NamedKey::F9 => logisim::input::Key::F9,
        NamedKey::F10 => logisim::input::Key::F10,
        NamedKey::F11 => logisim::input::Key::F11,
        NamedKey::F12 => logisim::input::Key::F12,
        _ => return None,
    })
}
