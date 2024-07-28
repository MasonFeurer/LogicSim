pub mod pages;
pub mod scene;

use glam::{vec2, Vec2};
use serde::{Deserialize, Serialize};

pub use egui::{Color32 as Color, Rect};

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
impl std::ops::Mul<egui::Vec2> for Transform {
    type Output = egui::Vec2;
    #[inline(always)]
    fn mul(self, v: egui::Vec2) -> egui::Vec2 {
        v * self.scale
    }
}
impl std::ops::Mul<egui::Pos2> for Transform {
    type Output = egui::Pos2;
    #[inline(always)]
    fn mul(self, v: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            v.x * self.scale + self.offset.x,
            v.y * self.scale + self.offset.y,
        )
    }
}
impl std::ops::Mul<f32> for Transform {
    type Output = f32;
    #[inline(always)]
    fn mul(self, r: f32) -> f32 {
        self.scale * r
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

pub fn create_visuals(theme: crate::settings::UiTheme) -> egui::Visuals {
    use crate::settings::UiTheme;
    use egui::{Color32, Rounding};
    let mut vis = egui::Visuals::default();
    vis.widgets.noninteractive.bg_stroke.width = 0.0;
    vis.widgets.inactive.bg_stroke.width = 0.0;
    vis.widgets.active.bg_stroke.width = 0.0;
    vis.widgets.hovered.bg_stroke.width = 0.0;

    match theme {
        UiTheme::Light => {
            // text color
            vis.widgets.noninteractive.fg_stroke.color = Color32::from_gray(0);
            vis.widgets.inactive.fg_stroke.color = Color32::from_gray(0);
            vis.widgets.hovered.fg_stroke.color = Color32::from_gray(100);
            vis.widgets.active.fg_stroke.color = Color32::from_gray(100);

            // menu background
            vis.panel_fill = Color32::from_gray(180);
            vis.window_fill = Color32::from_gray(180);
            // background
            vis.widgets.noninteractive.bg_fill = Color32::from_gray(215);
            // item rest color
            vis.widgets.inactive.bg_fill = Color32::from_gray(200);
            vis.widgets.inactive.weak_bg_fill = Color32::from_gray(200);
            // item hover color
            vis.widgets.hovered.bg_fill = Color32::from_gray(160);
            vis.widgets.hovered.weak_bg_fill = Color32::from_gray(160);
            // item interact color
            vis.widgets.active.bg_fill = Color32::from_gray(200);
            vis.widgets.active.weak_bg_fill = Color32::from_gray(200);

            // text edit background
            vis.extreme_bg_color = Color32::from_gray(220);

            vis.hyperlink_color = Color32::from_rgb(0, 0, 200);
            vis.selection.bg_fill = Color32::from_rgb(0, 0, 200);

            vis.widgets.inactive.rounding = Rounding::ZERO;
            vis.widgets.active.rounding = Rounding::ZERO;
            vis.widgets.hovered.rounding = Rounding::ZERO;
        }
        UiTheme::Dark => {
            // text color
            vis.widgets.noninteractive.fg_stroke.color = Color32::from_gray(255);
            vis.widgets.inactive.fg_stroke.color = Color32::from_gray(255);
            vis.widgets.hovered.fg_stroke.color = Color32::from_gray(180);
            vis.widgets.active.fg_stroke.color = Color32::from_gray(180);
            // menu background
            vis.panel_fill = Color32::from_gray(10);
            vis.window_fill = Color32::from_gray(10);
            // background
            vis.widgets.noninteractive.bg_fill = Color32::from_gray(5);
            // item rest color
            vis.widgets.inactive.bg_fill = Color32::from_gray(50);
            vis.widgets.inactive.weak_bg_fill = Color32::from_gray(50);
            // item hover color
            vis.widgets.hovered.bg_fill = Color32::from_gray(40);
            vis.widgets.hovered.weak_bg_fill = Color32::from_gray(40);
            // item interact color
            vis.widgets.active.bg_fill = Color32::from_gray(50);
            vis.widgets.active.weak_bg_fill = Color32::from_gray(50);

            // text edit background
            vis.extreme_bg_color = Color32::from_gray(0);

            vis.hyperlink_color = Color32::from_rgb(0, 0, 200);
            vis.selection.bg_fill = Color32::from_rgb(0, 0, 200);

            vis.widgets.inactive.rounding = Rounding::ZERO;
            vis.widgets.active.rounding = Rounding::ZERO;
            vis.widgets.hovered.rounding = Rounding::ZERO;
        }
    }
    vis
}
