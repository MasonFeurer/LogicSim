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

use std::collections::HashMap;
use std::time::SystemTime;
pub type Name = &'static str;

#[derive(Default)]
pub struct Perf {
    run_totals: HashMap<Name, u128>,
    frame_runs: HashMap<Name, u128>,
    current_run: Option<(Name, SystemTime)>,
    frame_count: u32,
}
impl Perf {
    pub fn start(&mut self, name: Name) {
        self.current_run = Some((name, SystemTime::now()));
    }
    pub fn end(&mut self) {
        let (name, start) = self.current_run.take().unwrap();
        let now = SystemTime::now();
        self.frame_runs
            .insert(name, now.duration_since(start).unwrap().as_millis());
    }
    pub fn end_frame(&mut self) {
        self.frame_count += 1;
        for (name, time) in &self.frame_runs {
            if let Some(total) = self.run_totals.get_mut(name) {
                *total += *time;
            } else {
                self.run_totals.insert(*name, *time);
            }
        }
        self.frame_runs.clear();
    }
    pub fn averages(&self) -> HashMap<Name, u128> {
        let frame_count = self.frame_count as u128;
        self.run_totals
            .iter()
            .map(|(name, time)| (*name, *time / frame_count))
            .collect()
    }
}
