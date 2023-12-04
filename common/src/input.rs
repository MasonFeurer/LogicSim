use crate::graphics::Rect;
use crate::Id;
use glam::{vec2, Vec2};

#[derive(Default, Clone, Debug, PartialEq)]
pub struct TextInputState {
    pub text: String,
    pub selection: std::ops::Range<u32>,
    pub compose: Option<std::ops::Range<u32>>,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct Modifiers {
    /// Any `SHIFT` is pressed
    pub shift: bool,

    /// * Windows - Any `ALT` key is pressed
    /// * Linux - Any `ALT` key is pressed
    /// * MacOS - Any `option` key is pressed
    pub option: bool,

    /// * Windows - Any `CTRL` key is pressed
    /// * Linux - Any `CTRL` key is pressed
    /// * MacOS - Any `command` key is pressed
    pub cmd: bool,

    /// * Windows - The `Windows` key is pressed
    /// * Linux - The `Super` key is pressed
    /// * MacOS - The `control` key is pressed
    pub os: bool,
}
impl Modifiers {
    pub fn update(&mut self, key: Key, state: bool) {
        match key {
            Key::Shift => self.shift = state,
            Key::Option => self.option = state,
            Key::Command => self.cmd = state,
            Key::Super => self.os = state,
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub struct Drag {
    pub button: PtrButton,
    pub id: Id,
    pub anchor: Vec2,
    pub press_pos: Vec2,
}

#[derive(Default, Clone)]
pub struct InputState {
    ptr_click: Option<(PtrButton, Vec2)>,
    ptr_press: Option<(PtrButton, Vec2)>,

    key_press: Option<Key>,
    char_press: Option<char>,

    ptr_pos: Option<Vec2>,
    prev_ptr_pos: Option<Vec2>,
    drag: Option<Drag>,

    modifiers: Modifiers,
    down_ptr_buttons: [bool; 5],
    text_input: Option<TextInputState>,
    scroll: Vec2,
    zoom: f32,
}
impl InputState {
    pub fn update_drag(&mut self, id: Id, bounds: Rect, anchor: Vec2, button: PtrButton) {
        self.update_drag_hovered(id, self.area_hovered(bounds), anchor, button)
    }

    pub fn update_drag_hovered(&mut self, id: Id, hovered: bool, anchor: Vec2, button: PtrButton) {
        if !hovered {
            return;
        }
        if let Some((b, press_pos)) = self.ptr_press {
            if b == button {
                self.drag = Some(Drag {
                    anchor,
                    id,
                    press_pos,
                    button,
                });
            }
        }
    }

    pub fn get_drag(&self, id: Id) -> Option<Vec2> {
        let Some(ptr_pos) = self.ptr_pos else {
            return None;
        };
        if let Some(drag) = &self.drag {
            if drag.id == id {
                return Some(drag.anchor + ptr_pos - drag.press_pos);
            }
        }
        None
    }
    pub fn get_drag_full(&self, id: Id) -> Option<Drag> {
        if self.ptr_pos.is_none() {
            return None;
        };
        if let Some(drag) = &self.drag {
            if drag.id == id {
                return Some(drag.clone());
            }
        }
        None
    }

    #[inline(always)]
    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll
    }
    #[inline(always)]
    pub fn zoom_delta(&self) -> f32 {
        self.zoom
    }

    pub fn any_changes(&self) -> bool {
        self.ptr_click.is_some()
            || self.ptr_press.is_some()
            || self.char_press.is_some()
            || self.key_press.is_some()
            || self.ptr_pos != self.prev_ptr_pos
    }

    // ---- Keyboard Input ----
    #[inline(always)]
    pub fn key_pressed(&self, key: Key) -> bool {
        self.key_press == Some(key)
    }
    #[inline(always)]
    pub fn char_press(&self) -> Option<char> {
        self.char_press
    }
    #[inline(always)]
    pub fn text_input(&self) -> Option<TextInputState> {
        self.text_input.clone()
    }
    #[inline(always)]
    pub fn set_text_input(&mut self, input: Option<TextInputState>) {
        self.text_input = input;
    }
    #[inline(always)]
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers.clone()
    }
    #[inline(always)]
    pub fn set_modifiers(&mut self, m: Modifiers) {
        self.modifiers = m;
    }

    // ---- Pointer Button Input ----
    #[inline(always)]
    pub fn ptr_down(&self, button: PtrButton) -> bool {
        self.down_ptr_buttons[usize::from(button)]
    }
    #[inline(always)]
    pub fn ptr_pressed(&self, button: PtrButton) -> bool {
        self.ptr_press.map(|press| press.0) == Some(button)
    }
    #[inline(always)]
    pub fn ptr_clicked(&self, button: PtrButton) -> bool {
        self.ptr_click.map(|click| click.0) == Some(button)
    }

    #[inline(always)]
    pub fn area_clicked(&self, area: Rect, button: PtrButton) -> bool {
        self.ptr_click
            .map(|(b, pos)| area.contains(pos) && b == button)
            == Some(true)
    }
    #[inline(always)]
    pub fn area_outside_clicked(&self, area: Rect, button: PtrButton) -> bool {
        self.ptr_click
            .map(|(b, pos)| !area.contains(pos) && b == button)
            == Some(true)
    }

    #[inline(always)]
    pub fn area_pressed(&self, area: Rect, button: PtrButton) -> bool {
        self.ptr_press
            .map(|(b, pos)| area.contains(pos) && b == button)
            == Some(true)
    }
    #[inline(always)]
    pub fn area_outside_pressed(&self, area: Rect, button: PtrButton) -> bool {
        self.ptr_press
            .map(|(b, pos)| !area.contains(pos) && b == button)
            == Some(true)
    }

    // ---- Pointer Location Input ----
    #[inline(always)]
    pub fn ptr_pos(&self) -> Vec2 {
        self.ptr_pos.unwrap_or(vec2(-1.0, -1.0))
    }
    #[inline(always)]
    pub fn ptr_gone(&self) -> bool {
        self.ptr_pos.is_none()
    }
    #[inline(always)]
    pub fn area_hovered(&self, area: Rect) -> bool {
        area.contains(self.ptr_pos())
    }
}
impl InputState {
    pub fn update(&mut self) {
        self.prev_ptr_pos = self.ptr_pos;
        self.ptr_click = None;
        self.ptr_press = None;
        self.key_press = None;
        self.char_press = None;
        self.zoom = 0.0;
        self.scroll = Vec2::ZERO;
    }

    pub fn on_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Click(pos, button) => self.ptr_click = Some((button, pos)),
            InputEvent::Press(pos, button) => {
                self.down_ptr_buttons[usize::from(button)] = true;
                self.ptr_press = Some((button, pos));
            }
            InputEvent::Release(_, button) => {
                if self.drag.as_ref().map(|drag| drag.button) == Some(button) {
                    self.drag = None;
                }
                self.down_ptr_buttons[usize::from(button)] = false;
            }
            InputEvent::Hover(pos) => self.ptr_pos = Some(pos),
            InputEvent::Type(ch) => self.char_press = Some(ch),
            InputEvent::PressKey(key) => {
                self.key_press = Some(key);
                self.modifiers.update(key, true);
            }
            InputEvent::ReleaseKey(key) => {
                self.modifiers.update(key, false);
            }
            InputEvent::PointerLeft => self.ptr_pos = None,
            InputEvent::Scroll(delta) => {
                if delta.x == 0.0 {
                    if self.modifiers.cmd {
                        self.zoom += delta.y;
                    } else if self.modifiers.shift {
                        self.scroll.x += delta.y;
                    } else {
                        self.scroll.y += delta.y;
                    }
                }
                self.scroll += delta;
            }
            InputEvent::Zoom(delta) => self.zoom += delta,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtrButton(u16);
impl PtrButton {
    pub const LEFT: Self = Self(0);
    pub const MIDDLE: Self = Self(1);
    pub const RIGHT: Self = Self(2);
    pub const FORWARD: Self = Self(3);
    pub const BACK: Self = Self(4);

    pub const fn new(v: u16) -> Self {
        Self(v)
    }
}
impl From<PtrButton> for u16 {
    fn from(b: PtrButton) -> u16 {
        b.0
    }
}
impl From<PtrButton> for usize {
    fn from(b: PtrButton) -> usize {
        b.0 as usize
    }
}

#[derive(Debug)]
pub enum InputEvent {
    Press(Vec2, PtrButton),
    Release(Vec2, PtrButton),
    Click(Vec2, PtrButton),
    Hover(Vec2),
    PressKey(Key),
    ReleaseKey(Key),
    Type(char),
    PointerLeft,
    Scroll(Vec2),
    Zoom(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[rustfmt::skip]
pub enum Key {
    Shift,
    Command,
    Option,
    Super,

    Backspace,
    Enter,
    Esc,
    Left,
    Right,
    Up,
    Down,
    Tab,
    Space,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
}
