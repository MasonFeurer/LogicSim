use crate::gpu::Gpu;
use crate::graphics::ui::{Align2, CycleState, MenuPainter, Painter, Style, TextField};
use crate::graphics::{Color, Model, Rect, Renderer, Transform, MAIN_ATLAS};
use crate::input::{InputState, PtrButton, TextInputState};
use crate::Id;

use crate::sim::save::{ChipSave, IoType, Library, SaveId};
use crate::sim::scene::{Chip as SceneChip, Rotation, Scene};
use crate::sim::{Node, NodeAddr, Sim, Source, SourceTy};

use glam::{UVec2, Vec2};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use serde::{Deserialize, Serialize};
use wgpu::*;

#[derive(Clone, Copy)]
pub enum ChipTy {
    Sequential,
    Combinational,
}
impl CycleState for ChipTy {
    fn advance(&mut self) {
        *self = match *self {
            Self::Sequential => Self::Combinational,
            Self::Combinational => Self::Sequential,
        }
    }
    fn label(&self) -> &'static str {
        match *self {
            Self::Sequential => "Sequential",
            Self::Combinational => "Combinational",
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum UiTheme {
    Light,
    Dark,
    Dracula,
    Neon,
    Pink,
}
impl CycleState for UiTheme {
    fn advance(&mut self) {
        *self = match *self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Dracula,
            Self::Dracula => Self::Neon,
            Self::Neon => Self::Pink,
            Self::Pink => Self::Light,
        }
    }
    fn label(&self) -> &'static str {
        match *self {
            Self::Light => "UI Theme: Light",
            Self::Dark => "UI Theme: Dark",
            Self::Dracula => "UI Theme: Dracula",
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
    fn advance(&mut self) {
        *self = match *self {
            Self::One => Self::Two,
            Self::Two => Self::Three,
            Self::Three => Self::One,
        }
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
            rs.text_color = Color::shade(0).into();
            rs.background = Color::shade(215);
            rs.menu_background = Color::shade(180).into();
            rs.item_color = Color::shade(100).into();
            rs.item_hover_color = Color::shade(160).into();
            rs.item_press_color = Color::shade(200).into();
        } else if matches!(self.ui_theme, UiTheme::Dracula) {
            rs.text_color = Color::shade(255).into();
            rs.menu_background = Color::shade(1).into();
            rs.background = Color::shade(0);
            rs.item_color = Color::shade(3).into();
            rs.item_hover_color = Color::shade(40).into();
            rs.item_press_color = Color::shade(2).into();
        } else if matches!(self.ui_theme, UiTheme::Neon) {
            rs.text_color = Color::shade(255).into();
            rs.menu_background = Color::shade(1).into();
            rs.background = Color::shade(0);
            rs.item_color = Color::rgb(0, 58, 58).into();
            rs.item_hover_color = Color::rgb(30, 80, 30).into();
            rs.item_press_color = Color::shade(1).into();
        } else if matches!(self.ui_theme, UiTheme::Pink) {
            rs.text_color = Color::shade(255).into();
            rs.menu_background = Color::rgb(38, 0, 22).into();
            rs.background = Color::shade(0);
            rs.item_color = Color::rgb(125, 0, 46).into();
            rs.item_hover_color = Color::rgb(122, 42, 71).into();
            rs.item_press_color = rs.menu_background;
        }
        rs
    }
}

#[derive(Clone, Copy)]
pub enum Cmd {
    None,
    SceneClear,
    SceneDel,
    SceneNew,
    ScenePack,
    PlacerAdd(usize),
    PlacerCancel,
    PlacerConfirm,
    ChipView(usize),
    ChipDelete(usize),
}

#[derive(Clone, Copy)]
pub enum Menu {
    Options,
    Info,
    Save,
    Library,
}

pub struct UiState {
    chip_ty: ChipTy,
    chip_name: TextField,
    library_sel: Option<usize>,
    open_menu: Option<Menu>,
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            chip_ty: ChipTy::Sequential,
            chip_name: TextField::default(),
            open_menu: None,
            library_sel: None,
        }
    }
}

pub fn show_info_menu(bounds: &mut Rect, p: &mut Painter, screen: Rect, state: &mut UiState) {
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
    out: &mut Vec<Cmd>,
) {
    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    p.text_lg(None, "Options");
    p.seperator();
    if p.button(None, "Info").clicked {
        state.open_menu = Some(Menu::Info);
    }
    p.cycle(None, &mut settings.ui_scale, &mut false);
    p.cycle(None, &mut settings.ui_theme, &mut false);
    if p.button(None, "Clear Scene").clicked {
        out.push(Cmd::SceneClear);
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
    out: &mut Vec<Cmd>,
) {
    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    p.text_lg(None, "Save to Chip");
    p.seperator();
    p.text_edit(None, "Name", &mut state.chip_name);
    p.cycle(None, &mut state.chip_ty, &mut false);
    if p.button(None, "Close").clicked {
        state.open_menu = None;
    }
    if p.button(None, "Create").clicked {
        out.push(Cmd::ScenePack)
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
    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    if let Some((idx, Some(chip))) = state.library_sel.map(|idx| (idx, chips.get(idx))) {
        p.text_lg(None, &chip.name);
        p.seperator();

        if p.button(None, "Delete").clicked {
            out.push(Cmd::ChipDelete(idx));
        }
        if chip.scene.is_some() {
            if p.button(None, "View").clicked {
                out.push(Cmd::ChipView(idx));
            }
        } else {
            p.text(None, "No scene available");
        }
        p.text(None, format!("L nodes: {}", chip.l_nodes.len()));
        p.text(None, format!("R nodes: {}", chip.r_nodes.len()));
        p.text(None, format!("Inner nodes: {}", chip.inner_nodes.len()));
        if p.button(None, "All").clicked {
            state.library_sel = None;
        }
    } else {
        p.text_lg(None, "Library");
        p.seperator();
        for (idx, chip) in chips.into_iter().enumerate() {
            if p.button(None, &chip.name).clicked {
                state.library_sel = Some(idx);
            }
        }
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
) {
    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.min, Align2::MIN, Align2::MIN, Vec2::X);

    if p.image_button(None, &MAIN_ATLAS["options_icon"]).clicked {
        state.open_menu = Some(Menu::Options);
    }
    if p.image_button(None, &MAIN_ATLAS["add_icon"]).clicked {
        state.open_menu = Some(Menu::Save);
    }
    p.text(None, format!("fps: {fps}"));
}

pub fn show_device_list(
    bounds: &mut Rect,
    p: &mut Painter,
    screen: Rect,
    state: &mut UiState,
    chips: &[ChipSave],
    out: &mut Vec<Cmd>,
) {
    let mut p = MenuPainter::new(bounds, p);
    let old_style = p.style().clone();
    p.style.item_size.x *= 0.5;

    p.start(screen.tr(), Align2::TOP_RIGHT, Align2::MIN, Vec2::Y);

    for (idx, chip) in chips.iter().enumerate() {
        let int = p.button(None, &chip.name);
        if int.clicked {
            out.push(Cmd::PlacerAdd(idx));
        } else if int.rclicked {
            state.open_menu = Some(Menu::Library);
            state.library_sel = Some(idx);
        }
    }
    p.style = old_style;
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
        let mut preview = chip.preview(tmp_pos, Rotation::Rot0);

        preview.pos.y += preview.size().y * 0.5;
        *stored_pos = preview.pos;
        let bounds = preview.bounds();
        p.set_transform(t);
        preview.draw(None, p, &mut Sim::default(), &mut Default::default());
        tmp_pos.y += preview.size().y + 5.0;

        if p.input.area_hovered(t * bounds) {
            *scene_hovered = false;
        }

        p.input.update_drag(
            Id::new("device_previews"),
            t * bounds,
            placer.pos,
            PtrButton::LEFT,
        );
        if let Some(drag) = p.input.get_drag_full(Id::new("device_previews")) {
            let offset = drag.press_pos - t * drag.anchor;
            placer.pos = t.inv() * (p.input.ptr_pos() - offset);
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

#[derive(Default)]
pub struct App {
    pub instance: Instance,
    pub gpu: Option<Gpu>,
    pub renderer: Option<Renderer>,

    pub settings: Settings,
    pub library: Library,
    pub scenes: Vec<Scene>,
    pub open_scene: usize,

    pub library_menu: Rect,
    pub options_menu: Rect,
    pub info_menu: Rect,
    pub save_menu: Rect,
    pub overlay_ui: Rect,
    pub device_list_ui: Rect,
    pub device_placer_ui: Rect,

    pub device_placer: DevicePlacer,
    pub ui_state: UiState,
    pub start_wire: Option<NodeAddr>,
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
        renderer.update_size(&gpu, win_size.as_vec2());
        renderer.update_global_transform(&gpu, Default::default());
        renderer.update_atlas_size(&gpu, MAIN_ATLAS.size);
        self.device_placer.pos = win_size.as_vec2() * 0.5;
        // self.scenes[self.open_scene]
        //     .init(Rect::from_min_size(Vec2::ZERO, win_size.as_vec2()));

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
                renderer.update_size(gpu, gpu.surface_size().as_vec2());
            }
        }
    }

    pub fn draw_frame(
        &mut self,
        input: &mut InputState,
        content_rect: Rect,
        text_input: &mut Option<TextInputState>,
        fps: u32,
    ) -> Result<(), String> {
        let gpu = self.gpu.as_ref().ok_or(format!("Missing Gpu instance"))?;
        let renderer = self
            .renderer
            .as_mut()
            .ok_or(format!("Missing Renderer instance"))?;

        // ---- Step Simulation ----
        self.scenes[self.open_scene]
            .sim
            .update(&self.library.tables);

        // ---- Handle Queued Events ----
        for cmd in self.commands.drain(..) {
            match cmd {
                Cmd::None => {}
                Cmd::SceneClear => self.scenes[self.open_scene].clear(),
                Cmd::SceneDel => {
                    self.scenes[self.open_scene].clear();
                    if self.scenes.len() > 1 {
                        self.scenes.remove(self.open_scene);
                        if self.open_scene > 0 {
                            self.open_scene -= 1;
                        }
                    }
                }
                Cmd::SceneNew => {
                    self.open_scene = self.scenes.len();
                    self.scenes.push(Scene::default());
                }
                Cmd::ScenePack => {
                    // self.scene.optimize();
                    let name = self.ui_state.chip_name.text.clone();
                    let save = create_chip_save(name, &self.scenes[self.open_scene]);
                    self.library.add(&[save]);
                    self.scenes[self.open_scene].clear();
                    self.ui_state.open_menu = None;
                }
                Cmd::PlacerAdd(idx) => self.device_placer.push_chip(idx as u32),
                Cmd::PlacerCancel => self.device_placer.clear(),
                Cmd::PlacerConfirm => {
                    let scene = &mut self.scenes[self.open_scene];
                    for (pos, chip_idx) in &self.device_placer.chips {
                        let chip = &self.library.chips[*chip_idx as usize];
                        place_chip(scene, None, chip, *pos, Rotation::Rot0);
                    }
                    self.device_placer.clear();
                }
                Cmd::ChipView(idx) => {
                    if let Some(scene) = &self.library.chips[idx].scene {
                        self.scenes.push(scene.clone());
                    }
                }
                Cmd::ChipDelete(idx) => _ = self.library.chips.remove(idx),
            }
        }

        // ------ Start Drawing ------
        let mut model = Model::default();
        let mut painter = Painter::new(self.settings.create_style(), input, &mut model);

        // ---- Draw Scene ----
        let show_device_placer_cond = self.device_placer.chips.len() > 0;
        let mut scene_hovered = !painter.input.area_hovered(self.device_list_ui)
            && !painter.input.area_hovered(self.overlay_ui)
            && self.ui_state.open_menu.is_none()
            && !(painter.input.area_hovered(self.device_placer_ui) && show_device_placer_cond);

        painter.set_transform(self.scenes[self.open_scene].transform);
        let scene_rs = self.scenes[self.open_scene].draw(&mut painter, &mut scene_hovered);
        if let Some(addr) = scene_rs.clicked_output {
            self.start_wire = Some(addr);
            log::info!("Started wire connection");
        }
        if let Some(addr) = scene_rs.clicked_input {
            if let Some(src_addr) = self.start_wire {
                log::info!("Placing wire");
                let src = if addr == src_addr {
                    Source::new_none()
                } else {
                    Source::new_copy(src_addr)
                };
                self.scenes[self.open_scene].sim.nodes[addr.0 as usize].set_source(src);
                self.start_wire = None;
            } else {
                log::info!("Toggling input");
                let state = self.scenes[self.open_scene].sim.nodes[addr.0 as usize].state();
                self.scenes[self.open_scene].sim.nodes[addr.0 as usize].set_state(!state);
            }
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

        // ---- Draw UI & Menus ----
        match self.ui_state.open_menu {
            Some(Menu::Options) => show_options_menu(
                &mut self.options_menu,
                &mut painter,
                content_rect,
                &mut self.ui_state,
                &mut self.settings,
                &mut self.commands,
            ),
            Some(Menu::Info) => show_info_menu(
                &mut self.info_menu,
                &mut painter,
                content_rect,
                &mut self.ui_state,
            ),
            Some(Menu::Save) => show_save_menu(
                &mut self.save_menu,
                &mut painter,
                content_rect,
                &mut self.ui_state,
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
                );
                show_device_list(
                    &mut self.device_list_ui,
                    &mut painter,
                    content_rect,
                    &mut self.ui_state,
                    &self.library.chips,
                    &mut self.commands,
                );
            }
        };

        // ---- Finish Drawing ----
        *text_input = painter.output.text_input.clone();
        let bg = Some(painter.style().background);
        renderer
            .render(gpu, bg, [&model.upload(&gpu.device)])
            .map_err(|e| format!("Failed to render to screen : {e:?}"))
    }
}

pub fn create_chip_save(name: String, scene: &Scene) -> ChipSave {
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
    for (_, device) in &scene.devices {
        for addr in device.sim_nodes() {
            inner_nodes.push((addr, scene.sim.get_node(addr)));
        }
    }
    ChipSave {
        region_size,
        name,
        scene: Some(scene.clone()),
        l_nodes,
        r_nodes,
        inner_nodes,
    }
}

pub fn place_chip(
    scene: &mut Scene,
    save_id: Option<SaveId>,
    save: &ChipSave,
    pos: Vec2,
    orientation: Rotation,
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
        region,
        pos,
        name: save.name.clone(),
        orientation,
        save: save_id,
        l_nodes,
        r_nodes,
        inner_nodes,
    };
    scene.add_device(chip);
}
