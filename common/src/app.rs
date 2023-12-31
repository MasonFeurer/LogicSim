use crate::gpu::Gpu;
use crate::graphics::ui::{Align2, CycleState, MenuPainter, Painter, Style, TextFieldAttrs};
use crate::graphics::{Color, ModelBuilder, Rect, Renderer, Transform, MAIN_ATLAS};
use crate::input::{InputState, Key, PtrButton, TextInputState};
use crate::Id;

use crate::sim::save::{ChipAttrs, ChipSave, IoType, ItemColor, Library};
use crate::sim::scene::{
    Chip as SceneChip, Device as SceneDevice, NodeIdent, Rotation, Scene, SceneId, Wire,
};
use crate::sim::{Node, NodeAddr, Sim, Source, SourceTy};

use glam::{vec2, UVec2, Vec2};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use serde::{Deserialize, Serialize};
use wgpu::*;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum UiTheme {
    Light,
    Dark,
    Night,
    Neon,
    Pink,
}
impl CycleState for UiTheme {
    fn from_u8(b: u8) -> Option<Self> {
        (b < 5).then(|| unsafe { std::mem::transmute(b) })
    }
    fn as_u8(&self) -> u8 {
        unsafe { std::mem::transmute(*self) }
    }
    fn label(&self) -> &'static str {
        match *self {
            Self::Light => "UI Theme: Light",
            Self::Dark => "UI Theme: Dark",
            Self::Night => "UI Theme: Night",
            Self::Neon => "UI Theme: Neon",
            Self::Pink => "UI Theme: Pink",
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum UiScale {
    One,
    Two,
    Three,
}
impl CycleState for UiScale {
    fn from_u8(b: u8) -> Option<Self> {
        (b < 3).then(|| unsafe { std::mem::transmute(b) })
    }
    fn as_u8(&self) -> u8 {
        unsafe { std::mem::transmute(*self) }
    }
    fn label(&self) -> &'static str {
        match *self {
            Self::One => "UI Scale: 1",
            Self::Two => "UI Scale: 2",
            Self::Three => "UI Scale: 3",
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub ui_scale: UiScale,
    pub ui_theme: UiTheme,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_scale: UiScale::One,
            ui_theme: UiTheme::Dark,
        }
    }
}
impl Settings {
    pub fn create_style(&self) -> Style {
        let mut rs = Style::default();
        let scale = match self.ui_scale {
            UiScale::One => 1.0,
            UiScale::Two => 2.0,
            UiScale::Three => 3.0,
        };

        rs.item_size *= scale;
        rs.text_size *= scale;
        rs.lg_text_size *= scale;
        rs.seperator_w *= scale;

        if matches!(self.ui_theme, UiTheme::Light) {
            rs.text_color = Color::shade(0);
            rs.background = Color::shade(215);
            rs.menu_background = Color::shade(180);
            rs.item_color = Color::shade(100);
            rs.item_hover_color = Color::shade(160);
            rs.item_press_color = Color::shade(200);
        } else if matches!(self.ui_theme, UiTheme::Night) {
            rs.text_color = Color::shade(255);
            rs.menu_background = Color::shade(1);
            rs.background = Color::shade(0);
            rs.item_color = Color::shade(3);
            rs.item_hover_color = Color::shade(40);
            rs.item_press_color = Color::shade(2);
        } else if matches!(self.ui_theme, UiTheme::Neon) {
            rs.text_color = Color::shade(255);
            rs.menu_background = Color::shade(1);
            rs.background = Color::shade(0);
            rs.item_color = Color::rgb(0, 58, 58);
            rs.item_hover_color = Color::rgb(30, 80, 30);
            rs.item_press_color = Color::shade(1);
        } else if matches!(self.ui_theme, UiTheme::Pink) {
            rs.text_color = Color::shade(255);
            rs.menu_background = Color::rgb(38, 0, 22);
            rs.background = Color::shade(0);
            rs.item_color = Color::rgb(125, 0, 46);
            rs.item_hover_color = Color::rgb(122, 42, 71);
            rs.item_press_color = rs.menu_background;
        }
        rs
    }
}

#[derive(Clone, Copy)]
pub enum Cmd {
    None,
    SceneSel(usize),
    SceneClear,
    SceneDel(usize),
    SceneNew,
    ScenePack,
    PlacerAdd(usize),
    PlacerCancel,
    PlacerConfirm,
    ChipView(usize),
    DeviceRem(SceneId),
    ChipDelete(usize),
    DownloadData,
    ImportData,
}

#[derive(Clone, Copy)]
pub enum Menu {
    Options,
    Info,
    Save,
    Library,
    Scenes,
}

#[derive(Default)]
pub struct UiState {
    save_name_field: TextFieldAttrs,
    chip_context: Option<SceneId>,
    library_sel: Option<usize>,
    open_menu: Option<Menu>,
    device_list_filter: Option<ItemColor>,
    device_list_closed: bool,
    ui_debugging: bool,
}

pub fn show_info_menu(bounds: &mut Rect, p: &mut Painter, screen: Rect, state: &mut UiState) {
    if p.input().area_outside_clicked(*bounds, PtrButton::LEFT) {
        state.open_menu = None;
        return;
    }

    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);
    p.text_lg(None, "Logisim");
    p.seperator();

    p.text(None, "Developer: Mason Feurer");
    p.text(None, "Version: 0.0.1 (dev)");
    let platform = format!(
        "Platform: {} ({})",
        std::env::consts::OS,
        std::env::consts::FAMILY
    );
    p.text(None, platform);
    p.text(None, format!("Architecture: {}", std::env::consts::ARCH));

    p.seperator();
    if p.button(None, "Done").clicked {
        state.open_menu = None;
    }
}

pub fn show_options_menu(
    bounds: &mut Rect,
    p: &mut Painter,
    screen: Rect,
    state: &mut UiState,
    settings: &mut Settings,
    external_data: bool,
    out: &mut Vec<Cmd>,
) {
    if p.input().area_outside_clicked(*bounds, PtrButton::LEFT) {
        state.open_menu = None;
        return;
    }

    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    p.text_lg(None, "Settings");
    p.seperator();
    if p.button(None, "Info").clicked {
        state.open_menu = Some(Menu::Info);
    }
    if p.button(None, "Library").clicked {
        state.open_menu = Some(Menu::Library);
    }
    p.cycle(None, &mut settings.ui_scale, &mut false);
    p.cycle(None, &mut settings.ui_theme, &mut false);
    if external_data && p.button(None, "Download Data").clicked {
        out.push(Cmd::DownloadData);
    }
    if external_data && p.button(None, "Import Data").clicked {
        out.push(Cmd::ImportData);
    }
    if p.button(None, "Done").clicked {
        state.open_menu = None;
    }
}

pub fn show_save_menu(
    bounds: &mut Rect,
    p: &mut Painter,
    screen: Rect,
    state: &mut UiState,
    attrs: &mut ChipAttrs,
    out: &mut Vec<Cmd>,
) {
    if p.input().area_outside_clicked(*bounds, PtrButton::LEFT) {
        state.open_menu = None;
        return;
    }

    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    p.text_lg(None, "Save to Chip");
    p.seperator();
    p.text_edit(None, "Name", state.save_name_field.as_mut(&mut attrs.name));

    p.push_style(|style| style.text_color = attrs.color.as_color());
    p.cycle(None, &mut attrs.color, &mut false);
    p.pop_style();

    p.cycle(None, &mut attrs.logic, &mut false);
    if p.button(None, "Close").clicked {
        state.open_menu = None;
    }
    if p.button(None, "Create").clicked {
        out.push(Cmd::ScenePack)
    }
}

pub fn show_scenes_menu(
    bounds: &mut Rect,
    p: &mut Painter,
    screen: Rect,
    state: &mut UiState,
    out: &mut Vec<Cmd>,
    scenes: &[Scene],
    open_scene: usize,
) {
    if p.input().area_outside_clicked(*bounds, PtrButton::LEFT) {
        state.open_menu = None;
        return;
    }

    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    p.text_lg(None, "Scenes");
    p.seperator();

    for (idx, scene) in scenes.iter().enumerate() {
        let label = match scene.save_attrs.name.is_empty() {
            true => "unnamed",
            false => &scene.save_attrs.name,
        };
        let label = match idx == open_scene {
            true => format!("[{label}]"),
            false => String::from(label),
        };
        let int = p.button(None, label);
        if int.clicked {
            out.push(Cmd::SceneSel(idx));
            state.open_menu = None;
        }
        if int.rclicked {
            out.push(Cmd::SceneDel(idx));
        }
    }

    p.seperator();
    if p.button(None, "New").clicked {
        out.push(Cmd::SceneNew);
        state.open_menu = None;
    }
    if p.button(None, "Clear").clicked {
        out.push(Cmd::SceneClear);
        state.open_menu = None;
    }
    if p.button(None, "Done").clicked {
        state.open_menu = None;
    }
}

pub fn show_library_menu(
    bounds: &mut Rect,
    p: &mut Painter,
    screen: Rect,
    state: &mut UiState,
    chips: &[ChipSave],
    out: &mut Vec<Cmd>,
) {
    if p.input().area_outside_clicked(*bounds, PtrButton::LEFT) {
        state.open_menu = None;
        return;
    }

    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    if let Some((idx, Some(chip))) = state.library_sel.map(|idx| (idx, chips.get(idx))) {
        p.text_lg(None, &chip.attrs.name);
        p.seperator();

        if chip.builtin {
            p.text(None, "Source: builtin");
        } else {
            p.text(None, "Source: user created");
        }

        p.text(None, format!("Logic: {}", chip.attrs.logic.label()));
        p.text(None, format!("L nodes: {}", chip.l_nodes.len()));
        p.text(None, format!("R nodes: {}", chip.r_nodes.len()));
        p.text(None, format!("Inner nodes: {}", chip.inner_nodes.len()));

        p.seperator();
        if chip.scene.is_some() && p.button(None, "Edit").clicked {
            out.push(Cmd::ChipView(idx));
        }
        if !chip.builtin && p.button(None, "Delete").clicked {
            out.push(Cmd::ChipDelete(idx));
        }
        if p.button(None, "Back").clicked {
            state.library_sel = None;
        }
    } else {
        p.text_lg(None, "Library");
        p.seperator();
        for (idx, chip) in chips.iter().enumerate() {
            if p.button(None, &chip.attrs.name).clicked {
                state.library_sel = Some(idx);
            }
        }
        p.seperator();
    }
    if p.button(None, "Close").clicked {
        state.open_menu = None;
    }
}

pub fn show_overlay(
    bounds: &mut Rect,
    p: &mut Painter,
    screen: Rect,
    fps: u32,
    state: &mut UiState,
    scene: &mut Scene,
) {
    // Menu buttons (top-left)
    let mut mp = MenuPainter::new(bounds, p);
    mp.start(screen.min, Align2::MIN, Align2::MIN, Vec2::X);

    if mp.image_button(None, &MAIN_ATLAS["settings_icon"]).clicked {
        state.open_menu = Some(Menu::Options);
    }
    if mp.image_button(None, &MAIN_ATLAS["scenes_icon"]).clicked {
        state.open_menu = Some(Menu::Scenes);
    }
    if mp.image_button(None, &MAIN_ATLAS["add_icon"]).clicked {
        state.open_menu = Some(Menu::Save);
    }

    let name = if scene.save_attrs.name.is_empty() {
        "unnamed"
    } else {
        &scene.save_attrs.name
    };
    mp.text(None, name);
    std::mem::drop(mp);

    // FPS Display (bottom-left, without menu)
    p.placer.bounds = Rect::default();
    p.start_placing(
        screen.bl() - Vec2::Y * p.style().text_size,
        Align2::TOP_LEFT,
        Align2::MIN,
        Vec2::X,
    );
    p.text(None, format!("{fps}fps"));
}

pub fn show_device_list(
    bounds: &mut Rect,
    p: &mut Painter,
    screen: Rect,
    state: &mut UiState,
    library: &Library,
    out: &mut Vec<Cmd>,
) {
    if p.input().key_pressed(Key::F4) {
        state.device_list_closed ^= true;
    }
    if state.device_list_closed {
        *bounds = Rect::default();
        let size = Vec2::splat(p.style().item_size.y);
        let bounds = Rect::from_min_size(screen.tr() - vec2(size.x, 0.0), size);
        if p.image_button(Some(bounds), &MAIN_ATLAS["point_l"]).clicked {
            state.device_list_closed = false;
        }
        return;
    }
    {
        let size = Vec2::splat(p.style().item_size.y);
        let bounds = Rect::from_min_size(screen.tr() - vec2(size.x + bounds.width(), 0.0), size);
        if p.image_button(Some(bounds), &MAIN_ATLAS["point_r"]).clicked {
            state.device_list_closed = true;
        }
    }

    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.tr(), Align2::TOP_RIGHT, Align2::MIN, Vec2::X);
    p.fit_button_text = true;

    let b_size = p.style().item_size.y * 0.5;

    for color in library.used_colors() {
        let selected = state.device_list_filter == Some(color);
        let center = p.placer.next(Vec2::splat(b_size)).center();
        let shape = Rect::from_center_size(center, Vec2::splat(b_size));
        let int = p.interact(shape);
        let ocolor = match selected {
            true => p.style().text_color,
            false => int.color,
        };
        if int.hovered || selected {
            p.model.circle(center, b_size * 0.6, 20, ocolor);
        }
        p.model
            .circle(center, b_size * 0.5, 20, color.as_color().inv().darken(30));
        p.model.circle(center, b_size * 0.45, 20, color.as_color());

        if int.clicked {
            match selected {
                true => state.device_list_filter = None,
                false => state.device_list_filter = Some(color),
            }
        }
    }
    p.start(
        screen.tr() + Vec2::Y * (b_size + p.style().item_spacing.y * 2.0),
        Align2::TOP_RIGHT,
        Align2::MIN,
        Vec2::Y,
    );
    p.push_style(|style| style.item_size *= 0.5);

    for (idx, chip) in library.chips.iter().enumerate() {
        if state.device_list_filter.is_some() && state.device_list_filter != Some(chip.attrs.color)
        {
            continue;
        }

        let int = p.button(None, &chip.attrs.name);
        if int.clicked {
            out.push(Cmd::PlacerAdd(idx));
        } else if int.rclicked {
            state.open_menu = Some(Menu::Library);
            state.library_sel = Some(idx);
        }
    }
    p.pop_style();
}

pub fn show_device_placer(
    bounds: &mut Rect,
    p: &mut Painter,
    t: Transform,
    placer: &mut DevicePlacer,
    library: &Library,
    out: &mut Vec<Cmd>,
    scene_hovered: &mut bool,
) {
    let pos = t * placer.pos;
    let mut mp = MenuPainter::new(bounds, p);
    mp.start(pos, Align2::BOTTOM_CENTER, Align2::MIN, Vec2::X);

    if mp.image_button(None, &MAIN_ATLAS["confirm_icon"]).clicked {
        out.push(Cmd::PlacerConfirm);
    }
    if mp.image_button(None, &MAIN_ATLAS["cancel_icon"]).clicked {
        out.push(Cmd::PlacerCancel);
    }
    std::mem::drop(mp);

    p.set_transform(t);
    let mut tmp_pos = placer.pos;
    for (stored_pos, chip_idx) in placer.chips.iter_mut() {
        let chip = &library.chips[*chip_idx as usize];
        let mut preview = chip.preview(tmp_pos, Rotation::default());

        preview.pos.y += preview.size().y * 0.5;
        *stored_pos = preview.pos;
        let bounds = preview.bounds();
        p.set_transform(t);
        preview.draw(None, p, &mut Sim::default(), &mut Default::default());
        tmp_pos.y += preview.size().y + 5.0;

        if p.input.area_hovered(t * bounds) {
            *scene_hovered = false;
        }

        if let Some(new_pos) = p.interact_drag(
            Id::new("device_previews"),
            bounds,
            placer.pos,
            PtrButton::LEFT,
        ) {
            placer.pos = new_pos;
        }
    }
}

#[derive(Default)]
pub struct DevicePlacer {
    chips: Vec<(Vec2, u32)>,
    pos: Vec2,
}
impl DevicePlacer {
    pub fn push_chip(&mut self, chip: u32) {
        self.chips.push((Vec2::ZERO, chip));
    }
    pub fn clear(&mut self) {
        self.chips.clear();
    }
}

pub fn show_chip_ctx_menu(
    bounds: &mut Rect,
    p: &mut Painter,
    pos: Vec2,
    id: SceneId,
    chip: &SceneChip,
    state: &mut UiState,
    out: &mut Vec<Cmd>,
    scene_hovered: &mut bool,
) {
    if p.input().area_outside_clicked(*bounds, PtrButton::LEFT) {
        state.chip_context = None;
        return;
    }
    if p.input().area_hovered(*bounds) {
        *scene_hovered = false;
    }
    let mut p = MenuPainter::new(bounds, p);
    p.start(pos, Align2::BOTTOM_CENTER, Align2::CENTER, Vec2::Y);

    p.text(None, &chip.attrs.name);
    p.seperator();
    if p.button(None, "Info").clicked {
        state.library_sel = chip.save;
        state.open_menu = Some(Menu::Library);
        state.chip_context = None;
    }
    if p.button(None, "Remove").clicked {
        out.push(Cmd::DeviceRem(id));
        state.chip_context = None;
    }
}

#[derive(Default)]
pub struct App {
    pub instance: Instance,
    pub gpu: Option<Gpu>,
    pub renderer: Option<Renderer>,
    pub external_data: bool,

    pub settings: Settings,
    pub library: Library,
    pub scenes: Vec<Scene>,
    pub open_scene: usize,

    pub library_menu: Rect,
    pub options_menu: Rect,
    pub info_menu: Rect,
    pub scenes_menu: Rect,
    pub save_menu: Rect,
    pub overlay_ui: Rect,
    pub device_list_ui: Rect,
    pub device_placer_ui: Rect,
    pub ctx_menu: Rect,

    pub device_placer: DevicePlacer,
    pub ui_state: UiState,
    pub start_wire: Option<(NodeIdent, NodeAddr)>,
    pub commands: Vec<Cmd>,
}
impl App {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.scenes.push(Scene::default());
        s
    }

    pub fn scene(&self) -> &Scene {
        &self.scenes[self.open_scene]
    }
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scenes[self.open_scene]
    }

    pub fn pause(&mut self) {
        self.renderer = None;
        self.gpu = None;
    }

    pub async fn resume<W: HasRawWindowHandle + HasRawDisplayHandle>(
        &mut self,
        window: &W,
        win_size: UVec2,
    ) {
        let gpu = Gpu::new(&self.instance, &window, win_size).await.unwrap();
        gpu.configure_surface();

        let mut renderer = Renderer::new(&gpu);
        renderer.locals.screen_size = win_size.as_vec2().into();
        renderer.locals.texture_size = MAIN_ATLAS.size;
        renderer.upload_locals(&gpu);

        self.device_placer.pos = self.scene().transform.inv() * (win_size.as_vec2() * 0.5);

        self.gpu = Some(gpu);
        self.renderer = Some(renderer);
    }

    pub fn size(&self) -> UVec2 {
        self.gpu
            .as_ref()
            .map(Gpu::surface_size)
            .unwrap_or(UVec2::ZERO)
    }

    pub fn update_size(&mut self, size: UVec2) {
        if let Some(gpu) = &mut self.gpu {
            gpu.surface_config.width = size.x;
            gpu.surface_config.height = size.y;
            gpu.configure_surface();

            if let Some(renderer) = &mut self.renderer {
                renderer.locals.screen_size = size.as_vec2().into();
                renderer.upload_locals(gpu);
            }
        }
        self.device_placer.pos = self.scene().transform.inv() * (size.as_vec2() * 0.5);
    }

    pub fn draw_frame(
        &mut self,
        input: &mut InputState,
        content_rect: Rect,
        text_input: &mut Option<TextInputState>,
        fps: u32,
        out: &mut FrameOutput,
    ) -> Result<(), String> {
        let gpu = self
            .gpu
            .as_ref()
            .ok_or(String::from("Missing Gpu instance"))?;
        let renderer = self
            .renderer
            .as_mut()
            .ok_or(String::from("Missing Renderer instance"))?;

        if let Some(d) = input.char_press().and_then(|ch| ch.to_digit(10)) {
            log::info!("Pressed digit: {d}");
        }

        // ---- Handle Key Binds ----
        // Note: Key binds than only work in specific UI elements are handled in the code for that UI.
        if input.key_pressed(Key::F1) {
            self.ui_state.open_menu = Some(Menu::Options);
        }
        if input.key_pressed(Key::F2) {
            self.ui_state.open_menu = Some(Menu::Scenes);
        }
        if input.key_pressed(Key::F3) && !input.modifiers().cmd {
            self.ui_state.open_menu = Some(Menu::Save);
        }
        if input.key_pressed(Key::Esc) {
            self.ui_state.open_menu = None;
        }
        if input.key_pressed(Key::F3) && input.modifiers().cmd {
            self.ui_state.ui_debugging ^= true;
        }

        // ---- Step Simulation ----
        self.scenes[self.open_scene]
            .sim
            .update(&self.library.tables);

        // ---- Handle Queued Events ----
        for cmd in self.commands.drain(..) {
            match cmd {
                Cmd::None => {}
                Cmd::SceneSel(idx) => self.open_scene = idx,
                Cmd::SceneClear => self.scenes[self.open_scene].clear(),
                Cmd::SceneDel(idx) => {
                    if self.scenes.len() > 1 {
                        self.scenes.remove(idx);
                        if self.open_scene >= self.scenes.len() {
                            self.open_scene = self.scenes.len() - 1;
                        }
                    }
                }
                Cmd::SceneNew => {
                    self.open_scene = self.scenes.len();
                    self.scenes.push(Scene::default());
                }
                Cmd::ScenePack => {
                    // self.scene.optimize();
                    let save = create_chip_save(&self.scenes[self.open_scene]);
                    self.scenes[self.open_scene].save_attrs = Default::default();

                    if let Some(c) = self
                        .library
                        .chips
                        .iter_mut()
                        .find(|chip| chip.attrs.name == save.attrs.name)
                    {
                        *c = save;
                    } else {
                        self.library.chips.push(save);
                    }

                    self.scenes[self.open_scene].clear();
                    self.ui_state.open_menu = None;
                }
                Cmd::PlacerAdd(idx) => self.device_placer.push_chip(idx as u32),
                Cmd::PlacerCancel => self.device_placer.clear(),
                Cmd::PlacerConfirm => {
                    let scene = &mut self.scenes[self.open_scene];
                    for (pos, chip_idx) in &self.device_placer.chips {
                        let chip = &self.library.chips[*chip_idx as usize];
                        place_chip(
                            scene,
                            Some(*chip_idx as usize),
                            chip,
                            *pos,
                            Rotation::default(),
                        );
                    }
                    self.device_placer.clear();
                }
                Cmd::ChipView(idx) => {
                    if let Some(scene) = &self.library.chips[idx].scene {
                        let idx = self.scenes.len();
                        self.scenes.push(scene.clone());
                        self.open_scene = idx;
                    }
                }
                Cmd::ChipDelete(idx) => {
                    let color = self.library.chips[idx].attrs.color;
                    if self.ui_state.device_list_filter == Some(color) {
                        self.ui_state.device_list_filter = None;
                    }
                    _ = self.library.chips.remove(idx);
                    if self.ui_state.library_sel == Some(idx) {
                        self.ui_state.library_sel = None;
                    }
                }
                Cmd::DeviceRem(id) => _ = self.scenes[self.open_scene].devices.remove(&id),
                Cmd::DownloadData => out.download_data = true,
                Cmd::ImportData => out.import_data = true,
            }
        }

        // ------ Start Drawing ------
        let mut model = ModelBuilder::default();
        let mut painter = Painter::new(self.settings.create_style(), input, &mut model);
        painter.debug = self.ui_state.ui_debugging;

        // ---- Draw Scene ----
        let show_device_placer_cond = !self.device_placer.chips.is_empty();
        let mut scene_hovered = !(self.ui_state.open_menu.is_some()
            || painter.input.area_hovered(self.overlay_ui)
            || painter.input.area_hovered(self.device_list_ui)
            || painter.input.area_hovered(self.device_placer_ui) && show_device_placer_cond);

        painter.set_transform(self.scenes[self.open_scene].transform);
        painter.covered = self.ui_state.open_menu.is_some();
        let scene_rs = self.scenes[self.open_scene].draw(&mut painter, &mut scene_hovered);
        painter.covered = false;

        if let Some(id) = scene_rs.rclicked_chip {
            self.ui_state.chip_context = Some(id);
        }

        // ---- Handle node clicks ----
        if let Some(addr) = scene_rs.toggle_node_state {
            let node = self.scenes[self.open_scene].sim.mut_node(addr);
            node.set_state(!node.state());
        }
        if let Some((ident, addr)) = scene_rs.clicked_node {
            if let Some((ident2, addr2)) = self.start_wire {
                if ident2 != ident {
                    self.start_wire = None;

                    let src = Source::new_copy(addr2);
                    self.scenes[self.open_scene].sim.nodes[addr.0 as usize].set_source(src);
                    self.scenes[self.open_scene].wires.push(Wire {
                        input: ident2,
                        output: ident,
                        anchors: vec![],
                    });
                }
            } else {
                self.start_wire = Some((ident, addr));
            }
        }

        // ---- Draw Chip Context Menu ----
        if let Some(id) = self.ui_state.chip_context {
            let scene = &self.scenes[self.open_scene];
            if let Some(SceneDevice::Chip(chip)) = scene.devices.get(&id) {
                let pos = scene.transform * (chip.pos - Vec2::Y * chip.size().y * 0.5);
                painter.set_transform(Transform::default());
                show_chip_ctx_menu(
                    &mut self.ctx_menu,
                    &mut painter,
                    pos,
                    id,
                    chip,
                    &mut self.ui_state,
                    &mut self.commands,
                    &mut scene_hovered,
                )
            }
        }

        // ---- Draw Device Placer ----
        if show_device_placer_cond {
            show_device_placer(
                &mut self.device_placer_ui,
                &mut painter,
                self.scenes[self.open_scene].transform,
                &mut self.device_placer,
                &self.library,
                &mut self.commands,
                &mut scene_hovered,
            );
        }

        // ---- Update Scene Pan + Zoom ----
        painter.input.update_drag_hovered(
            Id::new("background"),
            scene_hovered,
            self.scenes[self.open_scene].transform.offset,
            PtrButton::LEFT,
        );
        if let Some(new_offset) = painter.input.get_drag(Id::new("background")) {
            self.scenes[self.open_scene].transform.offset = new_offset;
        }
        if let Some((anchor, delta)) = painter.input.zoom_delta() {
            self.scenes[self.open_scene]
                .transform
                .zoom(anchor, delta * 0.1, 0.1..=100.0);
        }

        // ---- Draw UI & Menus ----
        match self.ui_state.open_menu {
            Some(Menu::Options) => show_options_menu(
                &mut self.options_menu,
                &mut painter,
                content_rect,
                &mut self.ui_state,
                &mut self.settings,
                self.external_data,
                &mut self.commands,
            ),
            Some(Menu::Info) => show_info_menu(
                &mut self.info_menu,
                &mut painter,
                content_rect,
                &mut self.ui_state,
            ),
            Some(Menu::Scenes) => show_scenes_menu(
                &mut self.scenes_menu,
                &mut painter,
                content_rect,
                &mut self.ui_state,
                &mut self.commands,
                &self.scenes,
                self.open_scene,
            ),
            Some(Menu::Save) => show_save_menu(
                &mut self.save_menu,
                &mut painter,
                content_rect,
                &mut self.ui_state,
                &mut self.scenes[self.open_scene].save_attrs,
                &mut self.commands,
            ),
            Some(Menu::Library) => show_library_menu(
                &mut self.library_menu,
                &mut painter,
                content_rect,
                &mut self.ui_state,
                &self.library.chips,
                &mut self.commands,
            ),
            None => {
                show_overlay(
                    &mut self.overlay_ui,
                    &mut painter,
                    content_rect,
                    fps,
                    &mut self.ui_state,
                    &mut self.scenes[self.open_scene],
                );
                show_device_list(
                    &mut self.device_list_ui,
                    &mut painter,
                    content_rect,
                    &mut self.ui_state,
                    &self.library,
                    &mut self.commands,
                );
            }
        };

        // ---- Finish Drawing ----
        *text_input = painter.output.text_input.clone();
        let bg = Some(painter.style().background);
        renderer
            .render(gpu, bg, [&model.finish(&gpu.device)])
            .map_err(|e| format!("Failed to render to screen : {e:?}"))
    }
}

pub fn create_chip_save(scene: &Scene) -> ChipSave {
    let region_size = scene.sim.next_region;
    let l_nodes = scene
        .l_nodes
        .states
        .iter()
        .map(|addr| (String::from(""), *addr, scene.sim.get_node(*addr)))
        .collect();
    let r_nodes = scene
        .r_nodes
        .states
        .iter()
        .map(|addr| (String::from(""), *addr, scene.sim.get_node(*addr)))
        .collect();
    let mut inner_nodes = Vec::new();
    for device in scene.devices.values() {
        for addr in device.sim_nodes() {
            inner_nodes.push((addr, scene.sim.get_node(addr)));
        }
    }
    ChipSave {
        attrs: scene.save_attrs.clone(),
        region_size,
        builtin: false,
        scene: Some(scene.clone()),
        l_nodes,
        r_nodes,
        inner_nodes,
    }
}

pub fn place_chip(
    scene: &mut Scene,
    save_id: Option<usize>,
    save: &ChipSave,
    pos: Vec2,
    rotation: Rotation,
) {
    let mut l_nodes = vec![];
    let mut r_nodes = vec![];
    let mut inner_nodes = vec![];
    let region = scene.sim.alloc_region(save.region_size);

    fn io_ty(node: &Node) -> IoType {
        match node.source().ty() {
            SourceTy::None => IoType::Input,
            _ => IoType::Output,
        }
    }

    for (name, addr, state) in &save.l_nodes {
        let addr = region.map(*addr);
        scene.sim.set_node(addr, region.map_node(*state));
        l_nodes.push((addr, name.clone(), io_ty(state)));
    }
    for (name, addr, state) in &save.r_nodes {
        let addr = region.map(*addr);
        scene.sim.set_node(addr, region.map_node(*state));
        r_nodes.push((addr, name.clone(), io_ty(state)));
    }
    for (addr, state) in &save.inner_nodes {
        let addr = region.map(*addr);
        scene.sim.set_node(addr, region.map_node(*state));
        inner_nodes.push(addr);
    }

    let chip = SceneChip {
        attrs: save.attrs.clone(),
        region,
        pos,
        rotation,
        save: save_id,
        l_nodes,
        r_nodes,
        inner_nodes,
    };
    scene.add_device(chip);
}

#[derive(Clone, Default)]
pub struct FrameOutput {
    pub download_data: bool,
    pub import_data: bool,
}
