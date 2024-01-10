pub mod atlas;
pub mod model;
pub mod renderer;
pub mod text;
pub mod ui;

pub use atlas::*;
pub use model::*;
pub use renderer::*;

use glam::{vec2, vec4, Vec2, Vec4};
use serde::{Deserialize, Serialize};

// http://www.sunshine2k.de/coding/java/PointOnLine/PointOnLine.html
pub fn project_point_onto_line(p: Vec2, line: (Vec2, Vec2)) -> Vec2 {
    let (v1, v2) = line;

    // get dot product of e1, e2
    let e1 = vec2(v2.x - v1.x, v2.y - v1.y);
    let e2 = vec2(p.x - v1.x, p.y - v1.y);
    let dot = e1.x * e2.x + e1.y * e2.y;

    // get squared length of e1
    let len_sq = e1.x * e1.x + e1.y * e1.y;

    let result_x = v1.x + (dot * e1.x) / len_sq;
    let result_y = v1.y + (dot * e1.y) / len_sq;
    vec2(result_x, result_y)
}
pub fn line_contains_point(line: (Vec2, Vec2), width: f32, point: Vec2) -> bool {
    let max_dist_sq = width * width;

    let projected = project_point_onto_line(point, line);

    let pp = projected - point;
    let dist_sq = (pp.x * pp.x + pp.y * pp.y).abs();

    let line_min_x = line.0.x.min(line.1.x);
    let line_max_x = line.0.x.max(line.1.x);
    let line_min_y = line.0.y.min(line.1.y);
    let line_max_y = line.0.y.max(line.1.y);

    dist_sq <= max_dist_sq
        && projected.x >= line_min_x
        && projected.x <= line_max_x
        && projected.y >= line_min_y
        && projected.y <= line_max_y
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
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

    pub const PINK: Self = Self(0xFC88A3FF);
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
    pub const fn r(self) -> u8 {
        ((self.0 & 0xFF000000) >> 24) as u8
    }
    #[inline(always)]
    pub const fn g(self) -> u8 {
        ((self.0 & 0x00FF0000) >> 16) as u8
    }
    #[inline(always)]
    pub const fn b(self) -> u8 {
        ((self.0 & 0x0000FF00) >> 8) as u8
    }
    #[inline(always)]
    pub const fn a(self) -> u8 {
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

    pub fn darken(self, v: u8) -> Self {
        Self::rgba(
            self.r().saturating_sub(v),
            self.g().saturating_sub(v),
            self.b().saturating_sub(v),
            self.a(),
        )
    }

    pub const fn inv(self) -> Self {
        Self::rgba(255 - self.r(), 255 - self.g(), 255 - self.b(), self.a())
    }
}

#[derive(Clone, Copy, Default)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
}
impl Stroke {
    pub fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
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
    pub fn translate(&mut self, v: Vec2) -> &mut Self {
        self.min += v;
        self.max += v;
        self
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub offset: Vec2,
    pub scale: f32,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            scale: 1.0,
        }
    }
}
impl std::ops::Mul<Vec2> for Transform {
    type Output = Vec2;
    #[inline(always)]
    fn mul(self, v: Vec2) -> Vec2 {
        v * self.scale + self.offset
    }
}
impl std::ops::Mul<Rect> for Transform {
    type Output = Rect;
    #[inline(always)]
    fn mul(self, r: Rect) -> Rect {
        let (min, max) = (self * r.min, self * r.max);
        Rect { min, max }
    }
}
impl Transform {
    #[inline(always)]
    pub fn from_offset(offset: Vec2) -> Self {
        Self { offset, scale: 1.0 }
    }

    #[inline(always)]
    pub fn inv(self) -> Self {
        let scale = 1.0 / self.scale;
        let offset = vec2(-self.offset.x / self.scale, -self.offset.y / self.scale);
        Self { scale, offset }
    }

    pub fn zoom(&mut self, pos: Vec2, delta: f32, range: std::ops::RangeInclusive<f32>) {
        if delta == 0.0 {
            return;
        }
        let xs = (pos.x - self.offset.x) / self.scale;
        let ys = (pos.y - self.offset.y) / self.scale;
        self.scale = (self.scale + delta).clamp(*range.start(), *range.end());

        self.offset.x = pos.x - xs * self.scale;
        self.offset.y = pos.y - ys * self.scale;
    }

    pub fn translate(&mut self, offset: Vec2) {
        self.offset += offset;
    }
}
