pub mod atlas;
pub mod model;
pub mod renderer;
pub mod text;
pub mod ui;

pub use atlas::*;
pub use model::*;
pub use renderer::*;
pub use text::*;

use crate::sim::NodeAddr;
use glam::{vec2, vec4, Vec2, Vec4};

#[derive(Clone, Copy, Default)]
pub struct Color(pub u32);
impl Color {
    pub const WHITE: Self = Self(0xFFFFFFFF);
    pub const BLACK: Self = Self(0x000000FF);

    pub const RED: Self = Self(0xFF0000FF);
    pub const GREEN: Self = Self(0x00FF00FF);
    pub const BLUE: Self = Self(0x0000FFFF);

    pub const YELLOW: Self = Self(0xFFFF00FF);
    pub const MAGENTA: Self = Self(0xFF00FFFF);
    pub const CYAN: Self = Self(0x00FFFFFF);

    pub const PINK: Self = Self(0xFF8383FF);
    pub const ORANGE: Self = Self(0xFF5F00FF);
    pub const MANGO: Self = Self(0xFF9900FF);

    #[inline(always)]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a as u32)
    }

    #[inline(always)]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    #[inline(always)]
    pub const fn shade(shade: u8) -> Self {
        Self::rgba(shade, shade, shade, 255)
    }

    #[inline(always)]
    pub fn r(self) -> u8 {
        ((self.0 & 0xFF000000) >> 24) as u8
    }
    #[inline(always)]
    pub fn g(self) -> u8 {
        ((self.0 & 0x00FF0000) >> 16) as u8
    }
    #[inline(always)]
    pub fn b(self) -> u8 {
        ((self.0 & 0x0000FF00) >> 8) as u8
    }
    #[inline(always)]
    pub fn a(self) -> u8 {
        (self.0 & 0x000000FF) as u8
    }

    pub fn as_vec4(self) -> Vec4 {
        vec4(
            self.r() as f32,
            self.g() as f32,
            self.b() as f32,
            self.a() as f32,
        ) / 255.0
    }
}

#[derive(Clone, Copy)]
pub enum ColorSrc {
    Node(NodeAddr),
    Set(Color),
}
impl ColorSrc {
    pub fn should_ignore(self) -> bool {
        match self {
            Self::Set(c) => c.a() == 0,
            _ => true,
        }
    }
}
impl Default for ColorSrc {
    fn default() -> Self {
        Self::Set(Color(0))
    }
}
impl From<Color> for ColorSrc {
    fn from(c: Color) -> Self {
        Self::Set(c)
    }
}
impl From<NodeAddr> for ColorSrc {
    fn from(addr: NodeAddr) -> Self {
        Self::Node(addr)
    }
}

#[derive(Clone, Copy, Default)]
pub struct Stroke<C> {
    pub width: f32,
    pub color: C,
}
impl<C> Stroke<C> {
    pub fn new(width: f32, color: C) -> Self {
        Self { width, color }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}
impl Rect {
    pub const ZERO: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    };

    #[inline(always)]
    pub const fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }
    #[inline(always)]
    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self {
            min,
            max: min + size,
        }
    }
    #[inline(always)]
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        Self {
            min: center - size * 0.5,
            max: center + size * 0.5,
        }
    }
    #[inline(always)]
    pub fn from_circle(center: Vec2, r: f32) -> Self {
        Self {
            min: center - Vec2::splat(r),
            max: center + Vec2::splat(r),
        }
    }

    #[inline(always)]
    pub fn expand(mut self, v: Vec2) -> Self {
        self.min -= v;
        self.max += v;
        self
    }

    #[inline(always)]
    pub fn shrink(mut self, v: Vec2) -> Self {
        self.min += v;
        self.max -= v;
        self
    }

    #[inline(always)]
    pub fn tl(&self) -> Vec2 {
        self.min
    }

    #[inline(always)]
    pub fn tr(&self) -> Vec2 {
        vec2(self.max.x, self.min.y)
    }

    #[inline(always)]
    pub fn br(&self) -> Vec2 {
        self.max
    }

    #[inline(always)]
    pub fn bl(&self) -> Vec2 {
        vec2(self.min.x, self.max.y)
    }

    #[inline(always)]
    pub fn translate(&mut self, v: Vec2) {
        self.min += v;
        self.max += v;
    }

    #[inline(always)]
    pub fn corners(&self) -> [Vec2; 4] {
        [
            vec2(self.min.x, self.min.y),
            vec2(self.max.x, self.min.y),
            vec2(self.max.x, self.max.y),
            vec2(self.min.x, self.max.y),
        ]
    }

    #[inline(always)]
    pub fn contains(&self, p: Vec2) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }

    #[inline(always)]
    pub fn expand_to_contain(&mut self, p: Vec2) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    #[inline(always)]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    #[inline(always)]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    #[inline(always)]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    #[inline(always)]
    pub fn center(&self) -> Vec2 {
        self.min + (self.max - self.min) * 0.5
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub offset: Vec2,
    pub scale: f32,
}
impl Transform {
    #[inline(always)]
    pub fn apply(self, v: Vec2) -> Vec2 {
        v * self.scale + self.offset
    }

    #[inline(always)]
    pub fn apply2(self, r: Rect) -> Rect {
        let (min, max) = (self.apply(r.min), self.apply(r.max));
        Rect { min, max }
    }

    #[inline(always)]
    pub fn from_offset(offset: Vec2) -> Self {
        Self { offset, scale: 1.0 }
    }
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PanZoomTransform {
    /// Screen coordinates of the origin in world space
    pub offset: Vec2,
    pub scale: f32,
    pub min_scale: f32,
    pub max_scale: f32,
}
impl Default for PanZoomTransform {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
            min_scale: 0.1,
            max_scale: 100.0,
        }
    }
}
impl PanZoomTransform {
    pub fn zoom(&mut self, pos: Vec2, delta: f32) {
        let xs = (pos.x - self.offset.x) / self.scale;
        let ys = (pos.y - self.offset.y) / self.scale;
        self.scale = (self.scale + delta).clamp(self.min_scale, self.max_scale);

        self.offset.x = pos.x - xs * self.scale;
        self.offset.y = pos.y - ys * self.scale;
    }
    pub fn pan(&mut self, offset: Vec2) {
        self.offset += offset;
    }

    pub fn transform(&self) -> Transform {
        let scale = self.scale;
        let offset = self.offset;
        Transform { scale, offset }
    }
    pub fn inv_transform(&self) -> Transform {
        let scale = 1.0 / self.scale;
        let offset = vec2(-self.offset.x / self.scale, -self.offset.y / self.scale);
        Transform { scale, offset }
    }
}
