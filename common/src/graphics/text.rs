use super::{Color, Image, ModelBuilder, Rect, StaticFont, MAIN_ATLAS};
use glam::{vec2, UVec2, Vec2};

/// ```rs
/// assert_eq!(split_first("Hello!"), Some(('H', "ello!")));
/// assert_eq!(split_first("😃🤣<-"), Some(('😃', "🤣<-")));
/// assert_eq!(split_first("😃"), Some(('😃', "")));
/// assert_eq!(split_first("x"), Some(('x', "")));
/// assert_eq!(split_first(""), None);
/// ```
fn split_first(s: &str) -> Option<(char, &str)> {
    if s.len() == 0 {
        return None;
    }
    let ch = s.chars().next().unwrap();
    let Some((start, _)) = s.char_indices().nth(1) else {
        return Some((ch, ""));
    };
    Some((ch, &s[start..]))
}

#[derive(Clone)]
pub struct TextLayoutGen<'a> {
    font: &'static StaticFont,
    text: &'a str,
    local_to_world: f32,
    cursor: Vec2,
    scale: f32,
    spacing: f32,
    offset: f32,
}
impl<'a> Iterator for TextLayoutGen<'a> {
    type Item = (char, Rect, Rect, Option<&'static Image>);
    fn next(&mut self) -> Option<Self::Item> {
        let Some((ch, remaining)) = split_first(self.text) else {
            return None;
        };
        self.text = remaining;

        if ch == ' ' || ch == '\t' {
            let w = match ch {
                ' ' => self.scale * 0.5,
                '\t' => self.scale * 1.5,
                _ => unreachable!(),
            };
            let min = self.cursor - vec2(0.0, self.scale + self.offset);
            let bounds = Rect::from_min_size(min, vec2(w, self.scale));
            self.cursor.x += w;
            return Some((ch, bounds, bounds, None));
        }

        let r = self.local_to_world;
        let img = self.font.get_char_image(ch);
        let offset = img.origin().as_vec2() * r;

        let real_min = self.cursor - vec2(0.0, offset.y + self.offset);
        let real_size = UVec2::from(img.size()).as_vec2() * r - Vec2::X * offset.x;
        let real_bounds = Rect::from_min_size(real_min, real_size);

        let img_size = UVec2::from(img.size()).as_vec2() * r;
        let img_min = self.cursor - vec2(offset.x, offset.y + self.offset);
        let img_bounds = Rect::from_min_size(img_min, img_size);

        self.cursor.x += real_size.x + self.spacing;
        Some((ch, real_bounds, img_bounds, Some(img)))
    }
}

pub fn layout_text(text: &str, scale: u32, start: Vec2) -> TextLayoutGen {
    let (font_key, font) = MAIN_ATLAS.get_font(scale, false, false);
    let local_to_world = scale as f32 / font_key.size as f32;
    TextLayoutGen {
        font,
        text,
        local_to_world,
        cursor: start + Vec2::Y * scale as f32,
        scale: (scale as f32) * 0.8,
        spacing: 0.0,
        offset: (scale as f32) * 0.2,
    }
}

pub fn text_size(text: &str, scale: u32) -> Vec2 {
    let mut max_x: f32 = 0.0;
    for (_ch, rect, _img_rect, _img) in layout_text(text, scale, Vec2::ZERO) {
        max_x = max_x.max(rect.max.x);
    }
    vec2(max_x, scale as f32)
}

pub fn build_text(model: &mut ModelBuilder, text: &str, scale: u32, start: Vec2, color: Color) {
    for (_ch, _rect, img_rect, img) in layout_text(text, scale, start) {
        if let Some(img) = img {
            model.rect(img_rect, img, color);
        }
    }
}
