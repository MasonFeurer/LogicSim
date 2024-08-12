#![windows_subsystem = "windows"]

use logisim::glam::{ivec2, uvec2, vec2, IVec2, UVec2};
use logisim::{app::App, egui, wgpu};
use logisim::{save::Project, settings::Settings, Platform};
use logisim_common as logisim;

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime};

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoopBuilder;
use winit::window::{Fullscreen, Window};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WindowSettings {
    pub pos: IVec2,
    pub size: UVec2,
    pub fullscreen: bool,
}

fn save_dir() -> PathBuf {
    let dirs = directories::ProjectDirs::from("com", "", "Logisim").unwrap();
    dirs.data_dir().to_owned()
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

static UI_SCALE: AtomicU32 = AtomicU32::new(100);

pub struct DesktopPlatform;
impl Platform for DesktopPlatform {
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
    fn can_open_dirs() -> bool { true }

    #[allow(unreachable_code)]
    fn open_save_dir() -> std::io::Result<()> {
        use std::process::Command;
        let dir = save_dir();

        log::info!("Attempting to open {dir:?}");

        #[cfg(target_os = "macos")]
        return Command::new("open").arg(&dir).spawn().map(|_| ());
        #[cfg(target_os = "windows")]
        return Command::new("explorer").arg(&dir).spawn().map(|_| ());
        #[cfg(target_os = "linux")]
        return Command::new("xdg-open").arg(&dir).spawn().map(|_| ());

        // If none of the above compile flags pass:
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Opending directory not implemented on this operating system",
        ))
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

    fn delete_project(name: &str) -> std::io::Result<()> {
        let name = format!("{name}.project");
        log::info!("Deleting {name}...");
        std::fs::remove_file(save_dir().join(name))
    }
    fn rename_project(name: &str, new_name: &str) -> std::io::Result<()> {
        let name = format!("{name}.project");
        let new_name = format!("{new_name}.project");
        log::info!("Renaming {name} to {new_name}...");
        std::fs::rename(save_dir().join(name), save_dir().join(new_name))
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
    fn is_touchscreen() -> bool { false }
	#[rustfmt::skip]
    fn has_physical_keyboard() -> bool { true }
	#[rustfmt::skip]
    fn name() -> String { "Desktop".into() }
}

fn set_fullscreen(window: &Window, fs: bool) {
    match fs {
        true => window.set_fullscreen(Some(Fullscreen::Borderless(None))),
        false => window.set_fullscreen(None),
    }
}
fn get_fullscreen(window: &Window) -> bool {
    window.fullscreen().is_some()
}

fn main() {
    env_logger::init();
    let event_loop = EventLoopBuilder::new().build().unwrap();
    let window = winit::window::WindowBuilder::new()
        .with_title("Logisim")
        .build(&event_loop)
        .unwrap();

    let viewport_id = egui::Context::default().viewport_id();

    let input = egui_winit::State::new(egui::Context::default(), viewport_id, &window, None, None);

    if let Ok(settings) = load_data::<WindowSettings>("window.data") {
        set_fullscreen(&window, settings.fullscreen);
        window.set_outer_position(PhysicalPosition::new(settings.pos.x, settings.pos.y));
        _ = window.request_inner_size(PhysicalSize::new(settings.size.x, settings.size.y));
    }

    let mut state = State {
        app: App::default(),
        input,
        window: Arc::new(window),
        wgpu: wgpu::Instance::new(Default::default()),
        last_frame_time: SystemTime::now(),

        frame_count: 0,
        last_fps_update: SystemTime::now(),
        fps: 0,
    };

    match DesktopPlatform::load_settings() {
        Ok(settings) => state.app.settings = settings,
        Err(err) => log::warn!("Failed to parse settings: {err:?}"),
    }

    _ = event_loop.run(move |event, event_loop| {
        let mut exit = false;
        on_event(&mut state, event, &mut exit);
        if exit {
            event_loop.exit();
        }
    });
}

struct State {
    app: App<DesktopPlatform>,
    wgpu: wgpu::Instance,
    window: Arc<Window>,
    input: egui_winit::State,
    last_frame_time: SystemTime,
    frame_count: u32,
    last_fps_update: SystemTime,
    fps: u32,
}

fn on_event(state: &mut State, event: Event<()>, exit: &mut bool) {
    match event {
        Event::Resumed => {
            let size = <[u32; 2]>::from(state.window.inner_size()).into();
            let surface = state.wgpu.create_surface(state.window.clone()).unwrap();

            pollster::block_on(state.app.renew_surface(&state.wgpu, surface, size));
            state.app.update_size(size);
            state.window.request_redraw();
        }
        Event::Suspended => log::info!("suspended"),
        Event::WindowEvent { event, .. } => on_window_event(state, event, exit),
        Event::LoopExiting => {
            _ = DesktopPlatform::save_settings(state.app.settings.clone());
            let size = state.window.inner_size();
            let pos = state.window.outer_position().unwrap_or(Default::default());
            let win_settings = WindowSettings {
                pos: ivec2(pos.x, pos.y),
                size: uvec2(size.width, size.height),
                fullscreen: get_fullscreen(&state.window),
            };
            _ = save_data("window.data", &win_settings);
        }
        _ => {}
    }
}

fn on_window_event(ctx: &mut State, event: WindowEvent, exit: &mut bool) {
    match event {
        event if ctx.input.on_window_event(&ctx.window, &event).consumed => {}
        WindowEvent::RedrawRequested => {
            let win_size = logisim::glam::uvec2(
                ctx.window.inner_size().width,
                ctx.window.inner_size().height,
            );
            let content_rect = egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(win_size.x as f32, win_size.y as f32),
            );

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

                ctx.last_frame_time = SystemTime::now();

                let mut input = logisim::app::AppInput {
                    egui_input: ctx.input.take_egui_input(&ctx.window),
                    fps: ctx.fps,
                    content_rect,
                    win_size,
                };

                // scaling
                {
                    let input_scale = UI_SCALE.load(Ordering::Relaxed) as f32 * 0.01;
                    let content_rect = {
                        let size = vec2(win_size.x as f32, win_size.y as f32);
                        let (min, max) = (vec2(0.0, 0.0), size / input_scale);
                        egui::Rect::from_min_max(egui::pos2(min.x, min.y), egui::pos2(max.x, max.y))
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

                match ctx.app.draw_frame(input) {
                    Ok(platform_output) => ctx
                        .input
                        .handle_platform_output(&ctx.window, platform_output),
                    Err(err) => log::warn!("Failed to draw frame: {err:?}"),
                }
            }
            ctx.window.request_redraw();
        }
        WindowEvent::Resized(_size) => {
            let size = <[u32; 2]>::from(ctx.window.inner_size()).into();
            ctx.app.update_size(size);
            ctx.window.request_redraw();
        }
        WindowEvent::CloseRequested => *exit = true,
        _ => {}
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
