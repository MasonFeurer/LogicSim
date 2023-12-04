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
pub use wgpu;

#[derive(Hash, Debug, Eq, PartialEq, Clone, Copy)]
pub struct Id(pub u64);
impl Id {
    pub fn new<T: std::hash::Hash>(v: T) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&v, &mut hasher);
        Self(std::hash::Hasher::finish(&hasher))
    }
}

pub unsafe fn slice_as_byte_slice<T>(a: &[T]) -> &[u8] {
    std::slice::from_raw_parts(a.as_ptr() as *const u8, a.len() * std::mem::size_of::<T>())
}
