#![windows_subsystem = "windows"]

use logisim::{app::App, egui, wgpu};
use logisim::{save::Project, settings::Settings};
use logisim_common as logisim;

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoopBuilder;
use winit::window::Window;

pub struct DesktopPlatform;
impl logisim::Platform for DesktopPlatform {
    fn load_settings() -> std::io::Result<Settings> {
        todo!()
    }
    fn save_settings(settings: Settings) -> std::io::Result<()> {
        todo!()
    }

    #[rustfmt::skip]
    fn can_open_projects_dir() -> bool { true }

    fn open_projects_dir() -> std::io::Result<()> {
        use std::process::Command;
        let dirs = directories::ProjectDirs::from("com", "", "Logisim").unwrap();
        let dir = dirs.data_dir().display().to_string();

        println!("Notic: Attempting to open {dir:?}");

        #[cfg(target_os = "macos")]
        return Command::new("open").arg(&dir).spawn().map(|_| ());
        #[cfg(target_os = "windows")]
        return Command::new("explorer").arg(&dir).spawn().map(|_| ());
        #[cfg(target_os = "linux")]
        return Command::new("xdg-open").arg(&dir).spawn().map(|_| ());
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Opending directory not implemented on this operating system",
        ))
    }

    fn list_available_projects() -> std::io::Result<Vec<String>> {
        let dirs = directories::ProjectDirs::from("com", "", "Logisim").unwrap();
        let dir = dirs.data_dir();

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
        let dirs = directories::ProjectDirs::from("com", "", "Logisim").unwrap();
        let dir = dirs.data_dir();
        let filename = format!("{name}.project");
        let bytes = std::fs::read(dir.join(filename))?;
        bincode::deserialize(&bytes).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to parse Project data from file",
            )
        })
    }
    fn save_project(name: &str, project: Project) -> std::io::Result<()> {
        let dirs = directories::ProjectDirs::from("com", "", "Logisim").unwrap();
        let dir = dirs.data_dir();
        _ = std::fs::create_dir(dir);
        let filename = format!("{name}.project");
        let bytes = bincode::serialize(&project).unwrap();
        std::fs::write(dir.join(filename), &bytes)
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
    fn is_touchscreen() -> bool { false }
	#[rustfmt::skip]
    fn has_physical_keyboard() -> bool { true }
	#[rustfmt::skip]
    fn name() -> String { "Desktop".into() }
}

struct SaveDirs {
    settings: PathBuf,
    library: PathBuf,
    scene: PathBuf,
}
impl SaveDirs {
    fn new() -> Self {
        let dirs = directories::ProjectDirs::from("com", "", "Logisim").unwrap();
        let dir = dirs.data_dir();
        _ = std::fs::create_dir(dir);
        Self {
            settings: dir.join("settings.data"),
            library: dir.join("library.data"),
            scene: dir.join("scene.data"),
        }
    }
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

    let mut state = State {
        app: App::default(),
        input,
        window,
        wgpu: wgpu::Instance::new(Default::default()),
        last_frame_time: SystemTime::now(),
        save_dirs: SaveDirs::new(),

        frame_count: 0,
        last_fps_update: SystemTime::now(),
        fps: 0,
    };

    if let Ok(bytes) = std::fs::read(&state.save_dirs.settings) {
        match bincode::deserialize(&bytes) {
            Ok(settings) => state.app.settings = settings,
            Err(err) => log::warn!("Failed to parse settings: {err:?}"),
        }
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
    window: Window,
    input: egui_winit::State,
    last_frame_time: SystemTime,
    save_dirs: SaveDirs,
    frame_count: u32,
    last_fps_update: SystemTime,
    fps: u32,
}

fn on_event(state: &mut State, event: Event<()>, exit: &mut bool) {
    match event {
        Event::Resumed => {
            let size = <[u32; 2]>::from(state.window.inner_size()).into();
            let surface = unsafe {
                use winit::raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
                state
                    .wgpu
                    .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                        raw_window_handle: state.window.raw_window_handle().unwrap(),
                        raw_display_handle: state.window.raw_display_handle().unwrap(),
                    })
                    .unwrap()
            };

            pollster::block_on(state.app.renew_surface(&state.wgpu, surface, size));
            state.app.update_size(size);
            state.window.request_redraw();
        }
        Event::Suspended => println!("suspended"),
        Event::WindowEvent { event, .. } => on_window_event(state, event, exit),
        Event::LoopExiting => {
            let settings = bincode::serialize(&state.app.settings).unwrap();
            match std::fs::write(&state.save_dirs.settings, settings) {
                Ok(_) => log::info!("Saved settings to {:?}", state.save_dirs.settings),
                Err(err) => log::warn!(
                    "Failed to save settings to {:?} : {err:?}",
                    state.save_dirs.settings
                ),
            }
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

                let input = logisim::app::AppInput {
                    egui_input: ctx.input.take_egui_input(&ctx.window),
                    fps: ctx.fps,
                    content_rect,
                    win_size,
                };

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
