use super::{Color, ColorSrc, GpuModel, Image, Model, Rect, Transform, MAIN_ATLAS};
use crate::gpu::Gpu;
use crate::input::{InputState, Key, PtrButton, TextInputState};
use glam::{vec2, Vec2};

#[derive(Clone)]
pub struct Style {
    pub text_size: f32,
    pub text_color: ColorSrc,
    pub background: Color,
    pub menu_background: ColorSrc,
    pub item_size: Vec2,
    pub item_color: ColorSrc,
    pub item_hover_color: ColorSrc,
    pub item_press_color: ColorSrc,
    pub item_spacing: Vec2,
    pub seperator_w: f32,
    pub margin: Vec2,
    pub item_align: Align,
    pub text_align: Align2,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            text_size: 30.0,
            text_color: Color::shade(255).into(),
            background: Color::shade(4),
            menu_background: Color::shade(10).into(),
            item_size: vec2(200.0, 35.0),
            item_color: Color::shade(30).into(),
            item_hover_color: Color::shade(20).into(),
            item_press_color: Color::shade(10).into(),
            item_spacing: vec2(5.0, 5.0),
            seperator_w: 3.0,
            margin: Vec2::splat(5.0),
            item_align: Align::Center,
            text_align: Align2::CENTER,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Dir {
    Px,
    Nx,
    Py,
    Ny,
}
impl Dir {
    pub const LEFT: Self = Self::Nx;
    pub const RIGHT: Self = Self::Px;
    pub const UP: Self = Self::Ny;
    pub const DOWN: Self = Self::Py;

    pub fn as_vec2(self) -> Vec2 {
        match self {
            Self::Px => vec2(1.0, 0.0),
            Self::Nx => vec2(-1.0, 0.0),
            Self::Ny => vec2(0.0, -1.0),
            Self::Py => vec2(0.0, 1.0),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Align {
    Min,
    Center,
    Max,
}

#[derive(Clone, Copy)]
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
    pub clicked_elsewhere: bool,
    pub color: ColorSrc,
    pub hovered: bool,
}

pub struct Painter<'a, 'b> {
    pub transform: Transform,
    pub style: &'a Style,
    pub input: &'a mut InputState,
    pub text_input: Option<TextInputState>,
    pub model: &'b mut Model,
    pub debug: bool,
}
impl<'a, 'b> Painter<'a, 'b> {
    pub fn new(style: &'a Style, input: &'a mut InputState, model: &'b mut Model) -> Self {
        Self {
            transform: Transform::default(),
            style,
            input,
            debug: false,
            text_input: None,
            model,
        }
    }

    pub fn set_transform(&mut self, t: Transform) {
        self.model.transform = t;
        self.transform = t;
    }
    pub fn reset_transform(&mut self) {
        self.transform = Transform::default();
        self.model.transform = Transform::default();
    }

    fn debug_shape(&mut self, shape: Rect) {
        if self.debug {
            self.model.rect_outline(shape, 2.0, Color::RED.into());
        }
    }

    pub fn interact(&mut self, shape: Rect) -> Interaction {
        let shape = self.transform.apply2(shape);
        let mut rs = Interaction::default();
        rs.hovered = self.input.area_hovered(shape);
        rs.color = if self.input.area_hovered(shape) {
            if self.input.ptr_down(PtrButton::LEFT) {
                self.style.item_press_color
            } else {
                self.style.item_hover_color
            }
        } else {
            self.style.item_color
        };
        rs.clicked = self.input.area_clicked(shape, PtrButton::LEFT);
        rs.clicked_elsewhere = self.input.area_outside_clicked(shape, PtrButton::LEFT);
        rs
    }

    pub fn menu_background(&mut self, shape: Rect) {
        let color = self.style.menu_background;
        self.model.rect(shape, &MAIN_ATLAS.white, color);
    }

    pub fn button<T: AsRef<str>>(&mut self, shape: Rect, label: T) -> ButtonRs {
        self.debug_shape(shape);
        button(shape, label, self)
    }
    pub fn image_button(&mut self, shape: Rect, tex: &Image) -> ButtonRs {
        self.debug_shape(shape);
        image_button(shape, tex, self)
    }
    pub fn text_edit<T: AsRef<str>>(&mut self, shape: Rect, hint: T, field: &mut TextField) {
        self.debug_shape(shape);
        text_edit(shape, hint, field, self)
    }
    pub fn cycle<State: CycleState>(&mut self, shape: Rect, state: &mut State, changed: &mut bool) {
        self.debug_shape(shape);
        cycle(shape, state, self, changed)
    }

    #[inline(always)]
    pub fn text_size(&self, text: impl AsRef<str>, scale: f32) -> Vec2 {
        super::text::text_size(text.as_ref(), scale as u32)
    }

    pub fn custom_text(
        &mut self,
        shape: Rect,
        text: impl AsRef<str>,
        color: ColorSrc,
        align: Align2,
    ) {
        let scale = shape.height();
        let size2 = self.text_size(&text, scale);
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
        super::text::build_text(
            &mut self.model,
            text.as_ref(),
            scale as u32,
            vec2(min_x, min_y),
            color,
        )
    }

    pub fn text<T: AsRef<str>>(&mut self, shape: Rect, text: T) {
        self.debug_shape(shape);
        self.custom_text(shape, text, self.style.text_color, self.style.text_align);
    }
    pub fn seperator(&mut self, bounds: Rect) {
        self.debug_shape(bounds);
        let w = bounds.height() * 0.5;
        let points = [bounds.min + vec2(0.0, w), bounds.max - vec2(0.0, w)];
        self.model
            .line(points, w, &MAIN_ATLAS.white, self.style.item_color);
    }

    pub fn upload(&self, gpu: &Gpu) -> GpuModel {
        self.model.upload(&gpu.device)
    }
}

pub struct Placer<'a> {
    pub anchor: Vec2,
    pub align_to_anchor: Align2,
    pub item_align: Align2,
    pub bounds: Rect,
    pub style: &'a Style,
    pub next_pos: Vec2,
    pub dir: Vec2,
    pub dirty: bool,
}
impl<'a> Placer<'a> {
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
    fn create_start_pos(style: &Style, item_align: Align2, bounds: Rect) -> Vec2 {
        let ui_bounds = bounds.shrink(style.margin);
        match item_align.x {
            Align::Min => vec2(ui_bounds.min.x, ui_bounds.min.y),
            Align::Max => vec2(ui_bounds.max.x, ui_bounds.min.y),
            Align::Center => ui_bounds.min + vec2(ui_bounds.width() * 0.5, 0.0),
        }
    }

    pub fn new(
        style: &'a Style,
        anchor: Vec2,
        align_to_anchor: Align2,
        item_align: Align2,
        dir: Vec2,
    ) -> Self {
        let bounds = Self::create_bounds(anchor, align_to_anchor, Vec2::ZERO);
        Self {
            anchor,
            align_to_anchor,
            item_align,
            bounds,
            dir,

            style,
            next_pos: Self::create_start_pos(style, item_align, bounds),
            dirty: true,
        }
    }

    pub fn start(&mut self) -> bool {
        if self.dirty {
            self.dirty = false;
            self.bounds =
                Self::create_bounds(self.anchor, self.align_to_anchor, self.bounds.size());
            self.next_pos = Self::create_start_pos(self.style, self.item_align, self.bounds);
            return true;
        }
        false
    }

    fn prepare_for_size(&mut self, size: Vec2) {
        let max = self.bounds.min + self.style.margin * 2.0 + size;
        if !self.bounds.contains(max) {
            self.bounds.expand_to_contain(max);
            self.dirty = true;
        }
    }

    fn rect_result(&mut self, rect: Rect) -> Rect {
        let max = rect.max + self.style.margin;
        if !self.bounds.contains(max) {
            self.dirty = true;
        }
        self.bounds.expand_to_contain(max);
        rect
    }

    pub fn next(&mut self) -> Rect {
        let anchor = self.next_pos;
        let size = self.style.item_size;
        self.prepare_for_size(size);
        let min = match self.item_align.x {
            Align::Min => anchor,
            Align::Center => vec2(anchor.x - size.x * 0.5, anchor.y),
            Align::Max => vec2(anchor.x - size.x, anchor.y),
        };
        self.next_pos += self.dir * (size + self.style.item_spacing);
        self.rect_result(Rect::from_min_size(min, size))
    }

    pub fn image_button(&mut self) -> Rect {
        let anchor = self.next_pos;
        let size = Vec2::splat(self.style.item_size.y);
        self.prepare_for_size(size);
        let min = match self.item_align.x {
            Align::Min => anchor,
            Align::Center => vec2(anchor.x - size.x * 0.5, anchor.y),
            Align::Max => vec2(anchor.x - size.x, anchor.y),
        };
        self.next_pos += self.dir * (size + self.style.item_spacing);
        self.rect_result(Rect::from_min_size(min, size))
    }

    pub fn seperator(&mut self) -> Rect {
        let anchor = self.next_pos;
        let size = vec2(self.style.item_size.x, self.style.seperator_w);
        self.prepare_for_size(size);
        let min = match self.item_align.x {
            Align::Min => anchor,
            Align::Center => vec2(anchor.x - size.x * 0.5, anchor.y),
            Align::Max => vec2(anchor.x - size.x, anchor.y),
        };
        self.next_pos += self.dir * (size + self.style.item_spacing);
        self.rect_result(Rect::from_min_size(min, size))
    }
}

// --------------- Widgets -----------------

#[derive(Default)]
pub struct ButtonRs {
    pub triggered: bool,
}
pub fn button<T: AsRef<str>>(shape: Rect, label: T, painter: &mut Painter) -> ButtonRs {
    let mut rs = ButtonRs::default();
    let Interaction { color, clicked, .. } = painter.interact(shape);

    painter.model.rect(shape, &MAIN_ATLAS.white, color);
    painter.text(shape, label);

    if clicked {
        rs.triggered = true;
    }
    rs
}

pub fn image_button(shape: Rect, tex: &Image, painter: &mut Painter) -> ButtonRs {
    let mut rs = ButtonRs::default();
    let Interaction { color, clicked, .. } = painter.interact(shape);
    painter.model.rect(shape, &MAIN_ATLAS.white, color);
    painter.model.rect(shape, tex, Color::WHITE.into());
    if clicked {
        rs.triggered = true;
    }
    rs
}

#[derive(Default)]
pub struct TextField {
    pub text: String,
    pub focused: bool,
    pub cursor: u32,
}
pub fn text_edit<T: AsRef<str>>(
    shape: Rect,
    hint: T,
    field: &mut TextField,
    painter: &mut Painter,
) {
    let hint = hint.as_ref();
    let Interaction {
        color,
        clicked,
        clicked_elsewhere,
        ..
    } = painter.interact(shape);

    if clicked {
        field.focused = true;
    } else if clicked_elsewhere {
        field.focused = false;
    }

    if field.focused {
        if painter.input.pasted_text().len() > 0 {
            field.text += painter.input.pasted_text();
        }
        if let Some(input) = &painter.input.text_input() {
            field.text = input.text.clone();
            field.cursor = input.selection.end;
        } else if painter.input.key_pressed(Key::Backspace) {
            if field.text.pop().is_some() {
                field.cursor -= 1;
            }
        } else if let Some(ch) = painter.input.char_press() {
            field.text.push(ch);
            field.cursor += 1;
        }
        painter.text_input = Some(TextInputState {
            text: field.text.clone(),
            selection: field.cursor..field.cursor,
            compose: Some(field.cursor..field.cursor + 1),
        });
    }

    field.cursor = field.text.len() as u32;
    painter.model.rect(shape, &MAIN_ATLAS.white, color);
    if field.focused {
        painter
            .model
            .rect_outline(shape, 2.0, painter.style.text_color);

        // CURSOR:
        let sc = shape.height();
        let char_idx = if field.cursor == 0 {
            0
        } else {
            field
                .text
                .char_indices()
                .nth(field.cursor as usize - 1)
                .unwrap_or((0, '\0'))
                .0
        };
        let text_before_cursor = if field.cursor == 0 {
            ""
        } else {
            &field.text[0..char_idx]
        };
        let offset = painter.text_size(text_before_cursor, sc).x;
        let min = shape.min + vec2(offset, sc * 0.05);
        let size = vec2(sc * 0.1, sc * 0.9);

        let color = painter.style.text_color;
        let rect = Rect::from_min_size(min, size);
        painter.model.rect(rect, &MAIN_ATLAS.white, color);
    }

    let text_color = match field.text.is_empty() {
        true => painter.style.item_press_color,
        false => painter.style.text_color,
    };
    let text: &str = match field.text.is_empty() {
        true => hint,
        false => &field.text,
    };

    if !text.is_empty() {
        painter.custom_text(shape, text, text_color, Align2::MIN);
    }
}

pub trait CycleState {
    fn advance(&mut self);
    fn label(&self) -> &'static str;
}
pub fn cycle<State>(shape: Rect, state: &mut State, painter: &mut Painter, changed: &mut bool)
where
    State: CycleState,
{
    let Interaction { clicked, color, .. } = painter.interact(shape);
    if clicked {
        state.advance();
        *changed = true;
    }
    painter.model.rect(shape, &MAIN_ATLAS.white, color);
    painter.text(shape, state.label());
}
