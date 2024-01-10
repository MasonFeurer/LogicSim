use crate::graphics::Rect;
use crate::Id;
use glam::Vec2;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct TextInputState {
    pub blink_timer: u128,
    pub id: Id,
    pub text: String,
    pub cursor: u32,
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
    visible_ptr_pos: Vec2,
    prev_ptr_pos: Option<Vec2>,
    drag: Option<Drag>,

    modifiers: Modifiers,
    pasted_text: String,
    down_ptr_buttons: [bool; 5],
    scroll: Vec2,
    zoom: Option<(Vec2, f32)>,

    pub millis: u128,
    pub active_text_field: Option<TextInputState>,
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
        self.ptr_pos?;
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
    pub fn zoom_delta(&self) -> Option<(Vec2, f32)> {
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
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers.clone()
    }
    #[inline(always)]
    pub fn set_modifiers(&mut self, m: Modifiers) {
        self.modifiers = m;
    }
    #[inline(always)]
    pub fn pasted_text(&self) -> &str {
        &self.pasted_text
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
    pub fn ptr_press(&self) -> Option<(PtrButton, Vec2)> {
        self.ptr_press
    }
    #[inline(always)]
    pub fn ptr_clicked(&self, button: PtrButton) -> bool {
        self.ptr_click.map(|click| click.0) == Some(button)
    }
    #[inline(always)]
    pub fn ptr_click(&self) -> Option<(PtrButton, Vec2)> {
        self.ptr_click
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
        self.ptr_pos.unwrap_or(self.visible_ptr_pos)
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
        self.pasted_text.clear();
        self.zoom = None;
        self.scroll = Vec2::ZERO;
    }

    fn add_zoom(&mut self, anchor: Vec2, delta: f32) {
        if let Some(zoom) = &mut self.zoom {
            if zoom.0 != anchor {
                log::warn!("Zoom anchor moved mid-frame ; not accounted for");
            }
            zoom.1 += delta;
        } else {
            self.zoom = Some((anchor, delta));
        }
    }

    pub fn on_event(&mut self, event: InputEvent) {
        // log::info!("Received event: {event:?}");
        match event {
            InputEvent::Paste(text) => self.pasted_text += &text,
            InputEvent::Click(pos, button) => self.ptr_click = Some((button, pos)),
            InputEvent::Press(pos, button) => {
                self.down_ptr_buttons[usize::from(button)] = true;
                self.ptr_press = Some((button, pos));
            }
            InputEvent::Release(button) => {
                if self.drag.as_ref().map(|drag| drag.button) == Some(button) {
                    self.drag = None;
                }
                self.down_ptr_buttons[usize::from(button)] = false;
            }
            InputEvent::Hover(pos) => {
                self.ptr_pos = Some(pos);
                self.visible_ptr_pos = pos;
            }
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
                        if let Some(pos) = self.ptr_pos {
                            self.add_zoom(pos, delta.y);
                        }
                    } else if self.modifiers.shift {
                        self.scroll.x += delta.y;
                    } else {
                        self.scroll.y += delta.y;
                    }
                }
                self.scroll += delta;
            }
            InputEvent::Zoom(anchor, delta) => self.add_zoom(anchor, delta),
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
    Release(PtrButton),
    Click(Vec2, PtrButton),
    Hover(Vec2),
    PressKey(Key),
    ReleaseKey(Key),
    Type(char),
    PointerLeft,
    Scroll(Vec2),
    Zoom(Vec2, f32),
    Paste(String),
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
