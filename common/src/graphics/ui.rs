use super::{Color, Image, ModelBuilder, Rect, Transform, MAIN_ATLAS};
use crate::input::{InputState, Key, PtrButton, TextInputState};
use crate::Id;
use glam::{vec2, Vec2};

#[derive(Clone)]
pub struct Style {
    pub text_size: f32,
    pub lg_text_size: f32,
    pub text_color: Color,
    pub background: Color,
    pub menu_background: Color,
    pub item_size: Vec2,
    pub item_color: Color,
    pub item_hover_color: Color,
    pub item_press_color: Color,
    pub item_spacing: Vec2,
    pub seperator_w: f32,
    pub margin: Vec2,
    pub button_margin: Vec2,
    pub item_align: Align,
    pub text_align: Align2,
    pub fit_button_text: bool,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            text_size: 30.0,
            lg_text_size: 50.0,
            text_color: Color::shade(255),
            background: Color::shade(4),
            menu_background: Color::shade(10),
            item_size: vec2(200.0, 35.0),
            item_color: Color::shade(30),
            item_hover_color: Color::shade(20),
            item_press_color: Color::shade(10),
            item_spacing: vec2(5.0, 5.0),
            seperator_w: 3.0,
            margin: Vec2::splat(5.0),
            button_margin: Vec2::splat(5.0),
            item_align: Align::Center,
            text_align: Align2::CENTER,
            fit_button_text: false,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum Align {
    #[default]
    Min,
    Center,
    Max,
}

#[derive(Clone, Copy, Default)]
pub struct Align2 {
    pub x: Align,
    pub y: Align,
}
impl Align2 {
    pub const MIN: Self = Self {
        x: Align::Min,
        y: Align::Min,
    };
    pub const TOP_LEFT: Self = Self {
        x: Align::Min,
        y: Align::Min,
    };
    pub const TOP_CENTER: Self = Self {
        x: Align::Min,
        y: Align::Center,
    };
    pub const TOP_RIGHT: Self = Self {
        x: Align::Max,
        y: Align::Min,
    };

    pub const CENTER_LEFT: Self = Self {
        x: Align::Min,
        y: Align::Center,
    };
    pub const CENTER: Self = Self {
        x: Align::Center,
        y: Align::Center,
    };
    pub const CENTER_RIGHT: Self = Self {
        x: Align::Max,
        y: Align::Center,
    };

    pub const BOTTOM_LEFT: Self = Self {
        x: Align::Min,
        y: Align::Max,
    };
    pub const BOTTOM_CENTER: Self = Self {
        x: Align::Center,
        y: Align::Max,
    };
    pub const BOTTOM_RIGHT: Self = Self {
        x: Align::Max,
        y: Align::Max,
    };

    pub fn origin(self, anchor: Vec2, size2: Vec2) -> Vec2 {
        vec2(
            match self.x {
                Align::Min => anchor.x,
                Align::Center => anchor.x - size2.x * 0.5,
                Align::Max => anchor.x - size2.x,
            },
            match self.y {
                Align::Min => anchor.y,
                Align::Center => anchor.y - size2.y * 0.5,
                Align::Max => anchor.y - size2.y,
            },
        )
    }
}

#[derive(Default, Clone)]
#[non_exhaustive]
pub struct Interaction {
    pub clicked: bool,
    pub rclicked: bool,
    pub clicked_elsewhere: bool,
    pub color: Color,
    pub hovered: bool,
}

#[derive(Default, Clone)]
pub struct Placer {
    pub anchor: Vec2,
    pub align_to_anchor: Align2,
    pub item_align: Align2,
    pub bounds: Rect,
    pub min_bounds: Rect,
    pub margin: Vec2,
    pub spacing: Vec2,
    pub cursor: Vec2,
    pub dir: Vec2,
    pub dirty: bool,
}
impl Placer {
    pub fn new(
        margin: Vec2,
        spacing: Vec2,
        anchor: Vec2,
        align_to_anchor: Align2,
        item_align: Align2,
        dir: Vec2,
        size: Vec2,
    ) -> Self {
        let bounds = Self::create_bounds(anchor, align_to_anchor, size);
        let min_bounds = Self::create_bounds(anchor, align_to_anchor, Vec2::ZERO);
        Self {
            margin,
            spacing,
            anchor,
            align_to_anchor,
            item_align,
            bounds,
            min_bounds,
            dir,
            cursor: Self::create_start_pos(margin, item_align, bounds),
            dirty: false,
        }
    }
}
impl Placer {
    pub fn set_size(&mut self, size: Vec2) {
        self.bounds = Self::create_bounds(self.anchor, self.align_to_anchor, size);
        self.min_bounds = Self::create_bounds(self.anchor, self.align_to_anchor, Vec2::ZERO);
        self.cursor = Self::create_start_pos(self.margin, self.item_align, self.bounds);
    }

    fn create_bounds(anchor: Vec2, align: Align2, size: Vec2) -> Rect {
        let x = match align.x {
            Align::Min => anchor.x,
            Align::Center => anchor.x - size.x * 0.5,
            Align::Max => anchor.x - size.x,
        };
        let y = match align.y {
            Align::Min => anchor.y,
            Align::Center => anchor.y - size.y * 0.5,
            Align::Max => anchor.y - size.y,
        };
        Rect::from_min_size(vec2(x, y), size)
    }
    fn create_start_pos(margin: Vec2, item_align: Align2, bounds: Rect) -> Vec2 {
        let ui_bounds = bounds.shrink(margin);
        match item_align.x {
            Align::Min => vec2(ui_bounds.min.x, ui_bounds.min.y),
            Align::Max => vec2(ui_bounds.max.x, ui_bounds.min.y),
            Align::Center => ui_bounds.min + vec2(ui_bounds.width() * 0.5, 0.0),
        }
    }

    fn rect_result(&mut self, rect: Rect) -> Rect {
        let rect2 = rect.expand(self.margin);
        if !self.bounds.contains(rect2.max) {
            self.dirty = true;
        }
        self.bounds.expand_to_contain(rect2.max);
        self.min_bounds.expand_to_contain(rect2.min);
        self.min_bounds.expand_to_contain(rect2.max);
        rect
    }

    pub fn next_unbounded(&mut self, size: Vec2) -> Rect {
        let anchor = self.cursor;
        let min = match self.item_align.x {
            Align::Min => anchor,
            Align::Center => vec2(anchor.x - size.x * 0.5, anchor.y),
            Align::Max => vec2(anchor.x - size.x, anchor.y),
        };
        self.cursor += self.dir * (size + self.spacing);
        Rect::from_min_size(min, size)
    }

    pub fn next(&mut self, size: Vec2) -> Rect {
        let anchor = self.cursor;
        let min = match self.item_align.x {
            Align::Min => anchor,
            Align::Center => vec2(anchor.x - size.x * 0.5, anchor.y),
            Align::Max => vec2(anchor.x - size.x, anchor.y),
        };
        self.cursor += self.dir * (size + self.spacing);
        self.rect_result(Rect::from_min_size(min, size))
    }
}

#[derive(Default, Clone)]
pub struct PainterOutput {
    pub text_input: Option<TextInputState>,
}

pub struct MenuPainter<'i, 'm, 'x, 'y> {
    bounds: &'x mut Rect,
    painter: &'y mut Painter<'i, 'm>,
    // The Vertex & Index position in the painter's model
    start: (u32, u32),
}
impl<'i, 'm, 'x, 'y> MenuPainter<'i, 'm, 'x, 'y> {
    pub fn new(bounds: &'x mut Rect, painter: &'y mut Painter<'i, 'm>) -> Self {
        let start = (
            painter.model.vertices.len() as u32,
            painter.model.indices.len() as u32,
        );
        painter.set_transform(Transform::default());
        painter.placer.set_size(bounds.size());
        Self {
            bounds,
            painter,
            start,
        }
    }

    pub fn start(&mut self, anchor: Vec2, align: Align2, item_align: Align2, dir: Vec2) {
        self.painter.start_placing(anchor, align, item_align, dir);
        self.painter.show_background();
    }
}
impl<'i, 'm, 'x, 'y> std::ops::Drop for MenuPainter<'i, 'm, 'x, 'y> {
    fn drop(&mut self) {
        *self.bounds = self.painter.placer.min_bounds;
        // If the placer is dirty, we should remove all the generated vertices and indices from the model.
        if self.painter.placer.dirty {
            self.painter.model.vertices.truncate(self.start.0 as usize);
            self.painter.model.indices.truncate(self.start.1 as usize);
        }
    }
}
impl<'i, 'm, 'x, 'y> std::ops::Deref for MenuPainter<'i, 'm, 'x, 'y> {
    type Target = Painter<'i, 'm>;
    fn deref(&self) -> &Self::Target {
        self.painter
    }
}
impl<'i, 'm, 'x, 'y> std::ops::DerefMut for MenuPainter<'i, 'm, 'x, 'y> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.painter
    }
}

pub struct Painter<'i, 'm> {
    pub covered: bool,
    pub debug: bool,
    pub transform: Transform,
    pub placer: Placer,
    pub style: Style,
    pub input: &'i mut InputState,
    pub model: &'m mut ModelBuilder,
}
impl<'i, 'm> Painter<'i, 'm> {
    pub fn new(style: Style, input: &'i mut InputState, model: &'m mut ModelBuilder) -> Self {
        Self {
            covered: false,
            debug: false,
            transform: Transform::default(),
            placer: Placer::default(),
            style,
            input,
            model,
        }
    }

    pub fn start_placing(&mut self, anchor: Vec2, align: Align2, item_align: Align2, dir: Vec2) {
        self.placer = Placer::new(
            self.style.margin,
            self.style.item_spacing,
            anchor,
            align,
            item_align,
            dir,
            self.placer.bounds.size(),
        );
    }

    pub fn set_placer(&mut self, p: Placer) {
        self.placer = p;
    }
    pub fn set_bounds(&mut self, b: Rect) {
        self.placer.bounds = b;
    }
    pub fn bounds(&self) -> Rect {
        self.placer.bounds
    }

    pub fn model_mut(&mut self) -> &mut ModelBuilder {
        self.model
    }
    pub fn style(&self) -> &Style {
        &self.style
    }
    pub fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }

    pub fn input(&self) -> &InputState {
        self.input
    }
    pub fn input_mut(&mut self) -> &mut InputState {
        self.input
    }

    pub fn set_transform(&mut self, t: Transform) {
        self.model.transform = t;
        self.transform = t;
    }

    pub fn interact_drag(
        &mut self,
        id: Id,
        bounds: Rect,
        anchor: Vec2,
        button: PtrButton,
    ) -> Option<Vec2> {
        if self.covered {
            return None;
        }
        self.input
            .update_drag(id, self.transform * bounds, anchor, button);
        if let Some(drag) = self.input.get_drag_full(id) {
            let offset = drag.press_pos - self.transform * drag.anchor;
            Some(self.transform.inv() * (self.input.ptr_pos() - offset))
        } else {
            None
        }
    }

    pub fn interact(&mut self, shape: Rect) -> Interaction {
        if self.covered {
            return Interaction {
                color: self.style.item_color,
                ..Default::default()
            };
        }
        let shape = self.transform * shape;
        Interaction {
            hovered: self.input.area_hovered(shape),
            color: if self.input.area_hovered(shape) {
                if self.input.ptr_down(PtrButton::LEFT) {
                    self.style.item_press_color
                } else {
                    self.style.item_hover_color
                }
            } else {
                self.style.item_color
            },
            rclicked: self.input.area_clicked(shape, PtrButton::RIGHT),
            clicked: self.input.area_clicked(shape, PtrButton::LEFT),
            clicked_elsewhere: self.input.area_outside_clicked(shape, PtrButton::LEFT),
        }
    }

    pub fn interact_hovered(&mut self, hovered: bool) -> Interaction {
        if self.covered {
            return Interaction {
                color: self.style.item_color,
                ..Default::default()
            };
        }
        Interaction {
            hovered,
            color: if hovered {
                if self.input.ptr_down(PtrButton::LEFT) {
                    self.style.item_press_color
                } else {
                    self.style.item_hover_color
                }
            } else {
                self.style.item_color
            },
            rclicked: hovered && self.input.ptr_clicked(PtrButton::RIGHT),
            clicked: hovered && self.input.ptr_clicked(PtrButton::LEFT),
            clicked_elsewhere: false, // placeholder value until InputState can check if some arbitrary shape has been clicked.
        }
    }

    pub fn interact_line(&mut self, line: [Vec2; 2], w: f32) -> Interaction {
        if self.covered {
            return Interaction {
                color: self.style.item_color,
                ..Default::default()
            };
        }
        let [a, b] = [self.transform * line[0], self.transform * line[1]];
        let hovered = super::line_contains_point((a, b), w, self.input.ptr_pos());
        Interaction {
            hovered,
            color: if hovered {
                if self.input.ptr_down(PtrButton::LEFT) {
                    self.style.item_press_color
                } else {
                    self.style.item_hover_color
                }
            } else {
                self.style.item_color
            },
            rclicked: hovered && self.input.ptr_clicked(PtrButton::RIGHT),
            clicked: hovered && self.input.ptr_clicked(PtrButton::LEFT),
            clicked_elsewhere: false, // placeholder value until InputState can check if some arbitrary shape has been clicked.
        }
    }
}
impl<'i, 'm> Painter<'i, 'm> {
    fn debug_shape(&mut self, shape: Rect) {
        if self.debug {
            self.model.rect_outline(shape, 2.0, Color::RED);
        }
    }

    pub fn show_background(&mut self) {
        self.model
            .rect(self.bounds(), &MAIN_ATLAS.white, self.style.menu_background)
    }

    pub fn button(&mut self, shape: Option<Rect>, label: impl AsRef<str>) -> Interaction {
        let shape = shape.unwrap_or_else(|| {
            let size = self.style.item_size;
            let text_size = self.text_size(&label, self.style.text_size);
            let size = size.max(text_size);
            self.placer.next(size)
        });
        self.debug_shape(shape);
        let int = self.interact(shape);
        self.model.rounded_rect(
            shape,
            shape.height() * 0.3,
            20,
            &MAIN_ATLAS.white,
            int.color,
        );
        let text_size = self.text_size(&label, self.style.text_size);
        self.place_text(
            shape,
            (label, text_size),
            self.style.text_color,
            Align2::CENTER,
        );
        int
    }
    pub fn circle_button(
        &mut self,
        center: Option<Vec2>,
        size: Option<f32>,
        label: impl AsRef<str>,
    ) -> Interaction {
        let size = Vec2::splat(size.unwrap_or(self.style.item_size.y));
        let center = center.unwrap_or_else(|| self.placer.next(size).center());
        let shape = Rect::from_center_size(center, size);
        self.debug_shape(shape);
        let int = self.interact(shape);
        self.model.circle(center, size.x * 0.5, 20, int.color);
        let text_size = self.text_size(&label, self.style.text_size);
        self.place_text(
            shape,
            (label, text_size),
            self.style.text_color,
            Align2::CENTER,
        );
        int
    }
    pub fn image_button(&mut self, shape: Option<Rect>, tex: &Image) -> Interaction {
        let shape = shape.unwrap_or_else(|| self.placer.next(Vec2::splat(self.style.item_size.y)));
        self.debug_shape(shape);
        let int = self.interact(shape);
        self.model.rounded_rect(
            shape,
            shape.height() * 0.3,
            20,
            &MAIN_ATLAS.white,
            int.color,
        );
        self.model
            .rect(shape.shrink(self.style.button_margin), tex, Color::WHITE);
        int
    }
    pub fn text_edit(
        &mut self,
        shape: Option<Rect>,
        id: Id,
        hint: impl AsRef<str>,
        text: &mut String,
    ) {
        let shape = shape.unwrap_or_else(|| self.placer.next(self.style.item_size));
        text_edit(shape, id, hint, text, self)
    }
    pub fn cycle<S: CycleState>(
        &mut self,
        shape: Option<Rect>,
        state: &mut S,
        changed: &mut bool,
    ) -> Interaction {
        let int = self.button(shape, state.label());
        if int.clicked {
            *state = S::from_u8(state.as_u8().wrapping_add(1)).unwrap_or(S::from_u8(0).unwrap());
            *changed = true;
        }
        int
    }

    pub fn toggle(
        &mut self,
        shape: Option<Rect>,
        label: impl AsRef<str>,
        state: &mut bool,
        changed: &mut bool,
    ) -> Interaction {
        let int = self.button(shape, label);
        if int.clicked {
            *state ^= true;
            *changed = true;
        }
        int
    }

    #[inline(always)]
    pub fn text_size(&self, text: impl AsRef<str>, scale: f32) -> Vec2 {
        super::text::text_size(text.as_ref(), scale as u32)
    }

    pub fn place_text(
        &mut self,
        shape: Rect,
        (text, size2): (impl AsRef<str>, Vec2),
        color: Color,
        align: Align2,
    ) {
        self.debug_shape(shape);
        let scale = size2.y;
        let min_x = match align.x {
            Align::Min => shape.min.x,
            Align::Center => shape.min.x + shape.width() * 0.5 - size2.x * 0.5,
            Align::Max => shape.max.x - size2.x,
        };
        let min_y = match align.y {
            Align::Min => shape.min.y,
            Align::Center => shape.min.y + shape.height() * 0.5 - size2.y * 0.5,
            Align::Max => shape.max.y - size2.y,
        };
        self.debug_shape(Rect::from_min_size(vec2(min_x, min_y), size2));
        super::text::build_text(
            self.model,
            text.as_ref(),
            scale as u32,
            vec2(min_x, min_y),
            color,
        )
    }

    pub fn text_lg(&mut self, bounds: Option<Rect>, text: impl AsRef<str>) {
        let size2 = self.text_size(&text, self.style.lg_text_size);
        let bounds = bounds.unwrap_or_else(|| self.placer.next(size2));
        self.place_text(bounds, (text, size2), self.style.text_color, Align2::MIN);
    }
    pub fn text(&mut self, bounds: Option<Rect>, text: impl AsRef<str>) {
        let size2 = self.text_size(&text, self.style.text_size);
        let bounds = bounds.unwrap_or_else(|| self.placer.next(size2));
        self.place_text(bounds, (text, size2), self.style.text_color, Align2::MIN);
    }
    pub fn seperator(&mut self) {
        let size = vec2(
            self.placer.bounds.width() - self.style.margin.x * 2.0,
            self.style.seperator_w,
        );
        let shape = self.placer.next_unbounded(size);
        self.debug_shape(shape);

        let w = shape.height() * 0.5;
        let points = [shape.min + vec2(0.0, w), shape.max - vec2(0.0, w)];
        self.model
            .line(points, w, &MAIN_ATLAS.white, self.style.item_color);
    }
}

pub trait CycleState {
    fn from_u8(b: u8) -> Option<Self>
    where
        Self: Sized;
    fn as_u8(&self) -> u8;
    fn label(&self) -> &'static str;
}

fn text_edit(shape: Rect, id: Id, hint: impl AsRef<str>, text: &mut String, g: &mut Painter) {
    // note: Most of this assumes an ASCII only string, which currently is the case,
    // but this will have to be redone if ever any plans to support more of UTF-8

    let hint = hint.as_ref();
    let Interaction {
        color,
        clicked,
        clicked_elsewhere,
        ..
    } = g.interact(shape);
    let mut active_field = g.input.active_text_field.clone();
    let mut is_focused = active_field.as_ref().map(|s| s.id == id) == Some(true);

    if clicked {
        active_field = Some(TextInputState {
            id,
            text: text.clone(),
            cursor: text.len() as u32,
            compose: None,
            blink_timer: g.input.millis,
        })
    } else if clicked_elsewhere && is_focused {
        active_field = None;
        is_focused = false;
    }

    if let Some(field) = &mut active_field {
        if is_focused {
            let insert_idx = field.cursor as usize;
            let mut reset_blinking = false;
            if !g.input.pasted_text().is_empty() {
                field.text += g.input.pasted_text();
                reset_blinking = true;
            }
            if g.input.key_pressed(Key::Backspace) {
                if insert_idx > 0 {
                    field.text.remove(insert_idx - 1);
                    field.cursor -= 1;
                }
                reset_blinking = true;
            } else if g.input.key_pressed(Key::Space) {
                field.text.insert(insert_idx, ' ');
                field.cursor += 1;
                reset_blinking = true;
            } else if g.input.key_pressed(Key::Tab) {
                field.text.insert(insert_idx, '\t');
                field.cursor += 1;
                reset_blinking = true;
            } else if g.input.key_pressed(Key::Left) {
                if field.cursor > 0 {
                    field.cursor -= 1;
                }
                reset_blinking = true;
            } else if g.input.key_pressed(Key::Right) {
                if field.cursor < field.text.len() as u32 {
                    field.cursor += 1;
                }
                reset_blinking = true;
            } else if g.input.key_pressed(Key::Home) {
                field.cursor = 0;
                reset_blinking = true;
            } else if g.input.key_pressed(Key::End) {
                field.cursor = (field.text.len()) as u32;
                reset_blinking = true;
            } else if let Some(ch) = g.input.char_press() {
                field.text.insert(insert_idx, ch);
                field.cursor += 1;
                reset_blinking = true;
            }
            if reset_blinking {
                field.blink_timer = g.input.millis;
            }
            *text = field.text.clone();
        }
    }
    g.input.active_text_field = active_field;
    g.model.rect(shape, &MAIN_ATLAS.white, color);
    if is_focused {
        g.model.rect_outline(shape, 2.0, g.style.text_color);

        let field = g.input.active_text_field.as_ref().unwrap();

        // --- Draw Cursor ----
        let cursor_byte_idx = field.cursor as usize;
        let text_before_cursor = &field.text[0..cursor_byte_idx];
        let cursor_offset = g.text_size(text_before_cursor, g.style.text_size).x;

        let cursor_rect = Rect::from_min_size(
            vec2(shape.min.x + cursor_offset, shape.min.y),
            vec2(2.0, g.style.text_size),
        );
        if g.input.millis - field.blink_timer < 400 {
            g.model
                .rect(cursor_rect, &MAIN_ATLAS.white, g.style.text_color);
        }
        if g.input.millis - field.blink_timer > 800 {
            g.input.active_text_field.as_mut().unwrap().blink_timer = g.input.millis;
        }
    }

    let text_color = match text.is_empty() {
        true => g.style.text_color.darken(120),
        false => g.style.text_color,
    };
    let text: &str = match text.is_empty() {
        true => hint,
        false => text,
    };

    if !text.is_empty() {
        let size = g.text_size(text, g.style.text_size);
        g.place_text(shape, (text, size), text_color, Align2::MIN);
    }
}
