pub mod app;
pub mod gpu;
pub mod settings;
pub mod sim;
pub mod ui;

pub use app::App;
pub use sim::save;

pub use egui;
pub use glam;
pub use log;
pub use wgpu;

use crate::save::Project;
use crate::settings::Settings;

#[derive(
    Default, Hash, Debug, Eq, PartialEq, Clone, Copy, serde::Serialize, serde::Deserialize,
)]
pub struct Id(pub u64);
impl Id {
    pub fn new<T: std::hash::Hash>(v: T) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&v, &mut hasher);
        Self(std::hash::Hasher::finish(&hasher))
    }
}

pub trait Platform {
    fn load_settings() -> std::io::Result<Settings>;
    fn save_settings(settings: Settings) -> std::io::Result<()>;

    fn list_available_projects() -> std::io::Result<Vec<String>>;
    fn load_project(name: &str) -> std::io::Result<Project>;
    fn save_project(name: &str, project: Project) -> std::io::Result<()>;

    fn can_open_projects_dir() -> bool;
    fn open_projects_dir() -> std::io::Result<()>;

    fn can_pick_file() -> bool;
    fn pick_file() -> impl std::future::Future<Output = std::io::Result<std::fs::File>> + Send;
    fn pick_files() -> impl std::future::Future<Output = std::io::Result<Vec<std::fs::File>>> + Send;

    fn has_external_data() -> bool;
    fn download_external_data();
    fn upload_external_data();

    fn is_touchscreen() -> bool;
    fn has_physical_keyboard() -> bool;
    fn name() -> String;
}
