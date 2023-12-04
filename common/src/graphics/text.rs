use super::{ColorSrc, Model, TexCoords};
use glam::{vec2, Vec2};
use rusttype::{GlyphId, Point, PositionedGlyph, Scale};

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

static DEFAULT_FONT_FILE: &[u8] = include_bytes!("../../include/open-sans-bold.ttf");

#[derive(PartialEq, Clone)]
pub struct GlyphKey(GlyphId, Scale, Point<f32>);
impl Hash for GlyphKey {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.0.hash(h);
        self.1.x.to_bits().hash(h);
        self.1.y.to_bits().hash(h);
        self.2.x.to_bits().hash(h);
        self.2.y.to_bits().hash(h);
    }
}
impl std::cmp::Eq for GlyphKey {}
impl From<&PositionedGlyph<'_>> for GlyphKey {
    fn from(g: &PositionedGlyph) -> Self {
        Self(g.id(), g.scale(), g.position())
    }
}

pub struct CachedGlyphModel {
    last_use: SystemTime,
    tris: Vec<[Vec2; 3]>,
}

pub struct Font<'a> {
    base: rusttype::Font<'a>,
    pub glyph_model_cache: HashMap<GlyphKey, CachedGlyphModel>,
    last_purge: SystemTime,
}
impl Default for Font<'static> {
    fn default() -> Self {
        Self::new(DEFAULT_FONT_FILE).unwrap()
    }
}
impl<'a> Font<'a> {
    pub fn new(bytes: &'a [u8]) -> Option<Self> {
        let base = rusttype::Font::try_from_bytes(bytes)?;
        Some(Self {
            base,
            glyph_model_cache: HashMap::new(),
            last_purge: SystemTime::now(),
        })
    }

    pub fn text_size(&self, text: &str, size: f32) -> Vec2 {
        let scale = rusttype::Scale::uniform(size);
        let h = self.base.v_metrics(scale).ascent;
        let mut w: f32 = 0.0;
        for g in self.base.layout(text, scale, rusttype::point(0.0, 0.0)) {
            w = w.max(g.position().x + g.unpositioned().h_metrics().advance_width);
        }
        vec2(w, h)
    }

    fn generate_glyph_model(builder: &mut TextBuilder, g: PositionedGlyph) -> Vec<[Vec2; 3]> {
        let pos = g.position();
        builder.clear();
        builder.set_offset(pos.x, pos.y);
        g.unpositioned().build_outline(builder);

        if builder.is_empty() {
            return vec![];
        }
        let Ok(triangles) = cdt::triangulate_contours(&builder.points, &builder.contours) else {
            return vec![];
        };
        let map_cdt_tri = |(a, b, c): (usize, usize, usize)| {
            let a = vec2(builder.points[a].0 as f32, builder.points[a].1 as f32);
            let b = vec2(builder.points[b].0 as f32, builder.points[b].1 as f32);
            let c = vec2(builder.points[c].0 as f32, builder.points[c].1 as f32);
            [a, b, c]
        };
        triangles.into_iter().map(map_cdt_tri).collect()
    }

    pub fn should_purge(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last_purge)
            .unwrap()
            .as_millis()
            > 2000
    }
    pub fn purge(&mut self) {
        let now = SystemTime::now();
        self.last_purge = now;
        let old: Vec<GlyphKey> = self
            .glyph_model_cache
            .iter()
            .filter(|(_key, c)| now.duration_since(c.last_use).unwrap().as_millis() > 500)
            .map(|(key, _c)| key.clone())
            .collect();
        for key in old {
            self.glyph_model_cache.remove(&key);
        }
    }

    pub fn build_text(
        &mut self,
        text: &str,
        offset: Vec2,
        size: f32,
        color: ColorSrc,
        detail: u32,
        mesh: &mut Model,
    ) {
        let scale = rusttype::Scale::uniform(size);
        let mut builder = TextBuilder::default();
        builder.detail = detail;
        builder.font_height = self.base.v_metrics(scale).ascent;
        for g in self
            .base
            .layout(text, scale, rusttype::point(offset.x, offset.y))
        {
            let key = GlyphKey::from(&g);

            let tris: &[[Vec2; 3]] = if let Some(c) = self.glyph_model_cache.get_mut(&key) {
                c.last_use = SystemTime::now();
                &c.tris
            } else {
                let tris = Self::generate_glyph_model(&mut builder, g);
                let c = CachedGlyphModel {
                    tris,
                    last_use: SystemTime::now(),
                };
                self.glyph_model_cache.insert(key.clone(), c);
                &self.glyph_model_cache.get(&key).unwrap().tris
            };
            for points in tris {
                mesh.tri(*points, &TexCoords::WHITE, color);
            }
        }
    }

    pub fn build_text_outline(
        &self,
        w: f32,
        text: &str,
        offset: Vec2,
        size: f32,
        color: ColorSrc,
        detail: u32,
        mesh: &mut Model,
    ) {
        let scale = rusttype::Scale::uniform(size);
        let mut builder = TextBuilder::default();
        builder.detail = detail;
        builder.font_height = self.base.v_metrics(scale).ascent;
        for g in self
            .base
            .layout(text, scale, rusttype::point(offset.x, offset.y))
        {
            let pos = g.position();
            builder.set_offset(pos.x, pos.y);
            g.unpositioned().build_outline(&mut builder);
        }

        for c in &builder.contours {
            for (a, b) in c.into_iter().zip(c.into_iter().skip(1)) {
                let a = vec2(builder.points[*a].0 as f32, builder.points[*a].1 as f32);
                let b = vec2(builder.points[*b].0 as f32, builder.points[*b].1 as f32);
                mesh.line([a, b], w, &TexCoords::WHITE, color);
            }
        }
    }
}

#[derive(Default)]
struct TextBuilder {
    points: Vec<(f64, f64)>,
    contours: Vec<Vec<usize>>,
    detail: u32,
    last_pos: Vec2,
    offset: Vec2,
    font_height: f32,
}
impl TextBuilder {
    fn clear(&mut self) {
        self.contours.clear();
        self.points.clear();
    }
    fn is_empty(&self) -> bool {
        self.points.is_empty() && self.contours.is_empty()
    }

    fn set_offset(&mut self, x: f32, y: f32) {
        self.offset = vec2(x, y);
    }

    fn add_point(&mut self, x: f32, mut y: f32) {
        self.last_pos = vec2(x, y);
        y += self.font_height;
        self.points
            .push(((x + self.offset.x) as f64, (self.offset.y + y) as f64));
    }
}
impl rusttype::OutlineBuilder for TextBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        // Begin a new contour
        self.contours.push(vec![self.points.len()]);
        self.add_point(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        // Push a move to this point into the last contour
        self.contours.last_mut().unwrap().push(self.points.len());
        self.add_point(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let Vec2 { x: x0, y: y0 } = self.last_pos;
        for i in 1..=self.detail {
            let t = i as f32 / (self.detail as f32);
            let f = |a, b, c| (1.0 - t).powf(2.0) * a + 2.0 * (1.0 - t) * t * b + t.powf(2.0) * c;
            self.line_to(f(x0, x1, x2), f(y0, y1, y2));
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        let Vec2 { x: x0, y: y0 } = self.last_pos;
        for i in 1..=self.detail {
            let t = i as f32 / (self.detail as f32);
            let f = |a, b, c, d| {
                (1.0 - t).powf(3.0) * a
                    + 3.0 * (1.0 - t).powf(2.0) * t * b
                    + 3.0 * (1.0 - t) * t.powf(2.0) * c
                    + t.powf(3.0) * d
            };
            self.line_to(f(x0, x1, x2, x3), f(y0, y1, y2, y3));
        }
    }

    fn close(&mut self) {
        // Remove last coordinate + point (which is a duplicate), then reassign
        let c = self.contours.last_mut().unwrap();
        *c.last_mut().unwrap() = c[0];
        self.points.pop().unwrap();

        // Leave position unchanged since we're going to start a new contour
        // shortly (if all is behaving well)
    }
}
