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
}
impl Default for Style {
    fn default() -> Self {
        Self {
            text_size: 30.0,
            lg_text_size: 50.0,
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
            button_margin: Vec2::splat(5.0),
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
    pub hovered: Option<Id>,
    pub text_input: Option<TextInputState>,
    pub drag_input: bool,
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
        &self.painter
    }
}
impl<'i, 'm, 'x, 'y> std::ops::DerefMut for MenuPainter<'i, 'm, 'x, 'y> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.painter
    }
}

pub struct Painter<'i, 'm> {
    pub covered: bool,
    pub transform: Transform,
    pub placer: Placer,
    pub style: Style,
    pub original_style: Option<Style>,
    pub input: &'i mut InputState,
    pub model: &'m mut ModelBuilder,
    pub output: PainterOutput,
    pub debug: bool,
    pub fit_button_text: bool,
}
impl<'i, 'm> Painter<'i, 'm> {
    pub fn new(style: Style, input: &'i mut InputState, model: &'m mut ModelBuilder) -> Self {
        Self {
            covered: false,
            transform: Transform::default(),
            placer: Placer::default(),
            style,
            original_style: None,
            input,
            model,
            output: PainterOutput::default(),
            debug: false,
            fit_button_text: false,
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
    pub fn push_style(&mut self, f: impl FnOnce(&mut Style)) {
        self.original_style = Some(self.style.clone());
        f(&mut self.style);
    }
    pub fn pop_style(&mut self) {
        if let Some(style) = self.original_style.take() {
            self.style = style;
        }
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
            let mut rs = Interaction::default();
            rs.color = self.style.item_color;
            return rs;
        }
        let shape = self.transform * shape;
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
        rs.rclicked = self.input.area_clicked(shape, PtrButton::RIGHT);
        rs.clicked = self.input.area_clicked(shape, PtrButton::LEFT);
        rs.clicked_elsewhere = self.input.area_outside_clicked(shape, PtrButton::LEFT);
        rs
    }
}
impl<'i, 'm> Painter<'i, 'm> {
    fn debug_shape(&mut self, shape: Rect) {
        if self.debug {
            self.model.rect_outline(shape, 2.0, Color::RED.into());
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
        self.model.rect(
            shape.shrink(self.style.button_margin),
            tex,
            Color::WHITE.into(),
        );
        int
    }
    pub fn text_edit(&mut self, shape: Option<Rect>, hint: impl AsRef<str>, field: TextFieldMut) {
        let shape = shape.unwrap_or_else(|| self.placer.next(self.style.item_size));
        text_edit(shape, hint, field, self)
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
            &mut self.model,
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

pub struct TextFieldMut<'a, 'b> {
    pub text: &'a mut String,
    pub focused: &'b mut bool,
    pub cursor: &'b mut u32,
}

#[derive(Default)]
pub struct TextFieldAttrs {
    pub focused: bool,
    pub cursor: u32,
}
impl<'b> TextFieldAttrs {
    pub fn as_mut<'a>(&'b mut self, text: &'a mut String) -> TextFieldMut<'a, 'b> {
        TextFieldMut {
            text,
            focused: &mut self.focused,
            cursor: &mut self.cursor,
        }
    }
}

#[derive(Default)]
pub struct TextField {
    pub text: String,
    pub focused: bool,
    pub cursor: u32,
}
impl<'a> TextField {
    pub fn as_mut(&'a mut self) -> TextFieldMut<'a, 'a> {
        TextFieldMut {
            text: &mut self.text,
            focused: &mut self.focused,
            cursor: &mut self.cursor,
        }
    }
}

fn text_edit(shape: Rect, hint: impl AsRef<str>, field_mut: TextFieldMut, g: &mut Painter) {
    let mut field = TextField {
        text: field_mut.text.clone(),
        focused: *field_mut.focused,
        cursor: *field_mut.cursor,
    };
    let hint = hint.as_ref();
    let Interaction {
        color,
        clicked,
        clicked_elsewhere,
        ..
    } = g.interact(shape);

    if clicked {
        field.focused = true;
    } else if clicked_elsewhere {
        field.focused = false;
    }

    if field.focused {
        if g.input.pasted_text().len() > 0 {
            field.text += g.input.pasted_text();
        }
        if let Some(input) = &g.input.text_input() {
            field.text = input.text.clone();
            field.cursor = input.selection.end;
        } else if g.input.key_pressed(Key::Backspace) {
            if field.text.pop().is_some() {
                field.cursor -= 1;
            }
        } else if g.input.key_pressed(Key::Space) {
            field.text.push(' ');
            field.cursor += 1;
        } else if g.input.key_pressed(Key::Tab) {
            field.text.push('\t');
            field.cursor += 1;
        } else if let Some(ch) = g.input.char_press() {
            field.text.push(ch);
            field.cursor += 1;
        }
        g.output.text_input = Some(TextInputState {
            text: field.text.clone(),
            selection: field.cursor..field.cursor,
            compose: Some(field.cursor..field.cursor + 1),
        });
    }

    field.cursor = field.text.len() as u32;
    g.model.rect(shape, &MAIN_ATLAS.white, color);
    if field.focused {
        g.model.rect_outline(shape, 2.0, g.style.text_color);

        // CURSOR:
        // let sc = shape.height();
        // let char_idx = if field.cursor == 0 {
        //     0
        // } else {
        //     field
        //         .text
        //         .char_indices()
        //         .nth(field.cursor as usize - 1)
        //         .unwrap_or((0, '\0'))
        //         .0
        // };
        // let text_before_cursor = if field.cursor == 0 {
        //     ""
        // } else {
        //     &field.text[0..char_idx]
        // };
        // let offset = painter.text_size(text_before_cursor, sc).x;
        // let min = shape.min + vec2(offset, sc * 0.05);
        // let size = vec2(sc * 0.1, sc * 0.9);

        // let color = painter.style.text_color;
        // let rect = Rect::from_min_size(min, size);
        // painter.model.rect(rect, &MAIN_ATLAS.white, color);
    }

    let text_color = match field.text.is_empty() {
        true => g.style.text_color.darken(120),
        false => g.style.text_color,
    };
    let text: &str = match field.text.is_empty() {
        true => hint,
        false => &field.text,
    };

    if !text.is_empty() {
        let size = g.text_size(text, g.style.text_size);
        g.place_text(shape, (text, size), text_color, Align2::MIN);
    }
    *field_mut.text = field.text.clone();
    *field_mut.focused = field.focused;
    *field_mut.cursor = field.cursor;
}
