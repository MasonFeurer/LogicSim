pub mod app;
pub mod gpu;
pub mod graphics;
pub mod input;
pub mod sim;

pub use app::App;
pub use gpu::*;
pub use graphics::*;
pub use sim::*;

pub use glam;
pub use log;
pub use wgpu;

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
