use logisim_common as logisim;

use logisim::app::{App, FrameOutput};
use logisim::glam::{uvec2, vec2, UVec2, Vec2};
use logisim::input::{InputEvent, InputState, PtrButton};
use logisim::{log, Rect};

use web_time::{Duration, SystemTime};

use wasm_bindgen::prelude::*;

use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::EventLoopBuilder;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

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

#[wasm_bindgen]
pub async fn main_web(canvas_id: &str) {
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

    log::info!("Starting app with size {size:?}");
    state.app.resume(&state.window, size).await;
    state.app.update_size(size);
    state.window.request_redraw();

    event_loop.spawn(move |event, event_loop| {
        let mut exit = false;
        on_event(&mut state, event, &mut exit);
        if exit {
            event_loop.exit();
        }
    });
}

fn on_event(state: &mut State, event: Event<()>, exit: &mut bool) {
    match event {
        Event::Resumed => log::info!("Received Resumed Event"),
        Event::Suspended => log::info!("Received Suspended Event"),
        Event::WindowEvent { event, .. } => on_window_event(state, event, exit),
        Event::LoopExiting => {
            let settings = bincode::serialize(&state.app.settings).unwrap();
            let library = bincode::serialize(&state.app.library).unwrap();
            let scene = bincode::serialize(&state.app.scene()).unwrap();
            (_, _, _) = (settings, library, scene);
            log::info!("TODO: SAVE APP STATE");
        }
        _ => {}
    }
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
                    log::info!("TODO: download data");
                }
                if out.import_data {
                    log::info!("TODO: import data");
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
            MouseScrollDelta::LineDelta(x, y) => ctx.input.on_event(InputEvent::Scroll(vec2(x, y))),
            MouseScrollDelta::PixelDelta(pos) => ctx
                .input
                .on_event(InputEvent::Scroll(vec2(pos.x as f32, pos.y as f32))),
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
