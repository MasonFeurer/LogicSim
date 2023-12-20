use crate::gpu::Gpu;
use crate::graphics::ui::{Align2, CycleState, MenuPainter, Painter, Style, TextField};
use crate::graphics::{Color, Model, Rect, Renderer, MAIN_ATLAS};
use crate::input::{InputState, PtrButton, TextInputState};
use crate::save::{ChipSave, Library};
use crate::sim::{self, save, scene, NodeAddr, Sim, Source};
use crate::Id;

use glam::{UVec2, Vec2};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use wgpu::*;

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub enum UiTheme {
    Light,
    Dark,
    Neon,
    Pink,
}
impl CycleState for UiTheme {
    fn advance(&mut self) {
        *self = match *self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Neon,
            Self::Neon => Self::Pink,
            Self::Pink => Self::Light,
        }
    }
    fn label(&self) -> &'static str {
        match *self {
            Self::Light => "UI Theme: Light",
            Self::Dark => "UI Theme: Dark",
            Self::Neon => "UI Theme: Neon",
            Self::Pink => "UI Theme: Pink",
        }
    }
}

#[derive(Debug, Clone, Copy)]
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

pub struct UiState {
    ui_size: UiScale,
    ui_theme: UiTheme,
    chip_ty: ChipTy,
    chip_name: TextField,
    open_menu: Option<Menu>,
    triggered_save: bool,
    clear_scene: bool,
    debug_ui: bool,
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            ui_size: UiScale::One,
            ui_theme: UiTheme::Dark,
            chip_ty: ChipTy::Sequential,
            chip_name: TextField::default(),
            open_menu: None,
            triggered_save: false,
            clear_scene: false,
            debug_ui: false,
        }
    }
}
impl UiState {
    pub fn create_style(&self) -> Style {
        let mut rs = Style::default();
        let scale = match self.ui_size {
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
            rs.background = Color::shade(255 - 8 * 5);
            rs.menu_background = Color::shade(255 - 10 * 5).into();
            rs.item_color = Color::shade(255 - 30 * 5).into();
            rs.item_hover_color = Color::shade(255 - 20 * 5).into();
            rs.item_press_color = Color::shade(255 - 10 * 5).into();
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

pub fn show_info_menu(bounds: &mut Rect, p: &mut Painter, screen: Rect, state: &mut UiState) {
    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);
    p.text_lg(None, "Info");
    p.seperator();

    p.text(None, "Developer: Mason Feurer");
    p.text(None, "Version: 0.0.1");
    let platform = format!(
        "Platform: {} ({})",
        std::env::consts::OS,
        std::env::consts::FAMILY
    );
    p.text(None, platform);
    p.text(None, format!("Architecture: {}", std::env::consts::ARCH));

    if p.button(None, "Done").clicked {
        state.open_menu = None;
    }
}

pub fn show_options_menu(bounds: &mut Rect, p: &mut Painter, screen: Rect, state: &mut UiState) {
    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    p.text_lg(None, "Options");
    p.seperator();
    if p.button(None, "Info").clicked {
        state.open_menu = Some(Menu::Info);
    }
    p.cycle(None, &mut state.ui_size, &mut false);
    p.cycle(None, &mut state.ui_theme, &mut false);
    p.toggle(None, "Debug UI", &mut state.debug_ui, &mut false);
    if p.button(None, "Clear The Scene").clicked {
        state.clear_scene = true;
    }
    if p.button(None, "Done").clicked {
        state.open_menu = None;
    }
}

pub fn show_save_menu(bounds: &mut Rect, p: &mut Painter, screen: Rect, state: &mut UiState) {
    let mut p = MenuPainter::new(bounds, p);
    p.start(screen.center(), Align2::CENTER, Align2::CENTER, Vec2::Y);

    p.text_lg(None, "Save to Chip");
    p.seperator();
    p.text_edit(None, "Name", &mut state.chip_name);
    p.cycle(None, &mut state.chip_ty, &mut false);
    if p.button(None, "Cancel").clicked {
        state.open_menu = None;
    }
    if p.button(None, "Done").clicked {
        state.triggered_save = true;
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
    chips: &[save::ChipSave],
    hold_chip: &mut Option<usize>,
) {
    let mut p = MenuPainter::new(bounds, p);
    let old_style = p.style().clone();
    p.style.item_size.x *= 0.5;

    p.start(screen.tr(), Align2::TOP_RIGHT, Align2::MIN, Vec2::Y);

    for (idx, chip) in chips.iter().enumerate() {
        if p.button(None, &chip.name).clicked {
            *hold_chip = Some(idx);
        }
    }
    p.style = old_style;
}

pub fn show_device_preview_header(
    bounds: &mut Rect,
    p: &mut Painter,
    pos: Vec2,
    confirm: &mut bool,
    cancel: &mut bool,
) {
    let mut p = MenuPainter::new(bounds, p);
    p.start(pos, Align2::BOTTOM_CENTER, Align2::MIN, Vec2::X);

    if p.image_button(None, &MAIN_ATLAS["confirm_icon"]).clicked {
        *confirm = true;
    }
    if p.image_button(None, &MAIN_ATLAS["cancel_icon"]).clicked {
        *cancel = true;
    }
}

pub enum Menu {
    Options,
    Info,
    Save,
}

#[derive(Default)]
pub struct App {
    pub instance: Instance,
    pub gpu: Option<Gpu>,
    pub renderer: Option<Renderer>,
    pub sim: Sim,

    pub scene: scene::Scene,

    pub options_menu: Rect,
    pub info_menu: Rect,
    pub save_menu: Rect,
    pub overlay_ui: Rect,
    pub device_list_ui: Rect,
    pub place_chips_ui: Rect,

    pub library: Library,
    pub chips_in_hand: Vec<u32>,
    pub ui_state: UiState,
    pub place_chips_pos: Vec2,
    pub start_wire: Option<NodeAddr>,
}
impl App {
    pub fn new() -> Self {
        let mut app = Self::default();
        app.sim.tables = app.library.tables.clone();
        app
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
        self.place_chips_pos = win_size.as_vec2() * 0.5;
        self.scene
            .init(Rect::from_min_size(Vec2::ZERO, win_size.as_vec2()));

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

        if self.ui_state.triggered_save {
            self.ui_state.triggered_save = false;
            // self.scene.optimize(&mut self.sim);
            let name = self.ui_state.chip_name.text.clone();
            let save = create_chip_save(name, &self.sim, &self.scene);
            self.library.add(&[save]);
            self.ui_state.clear_scene = true;
        }
        if self.ui_state.clear_scene {
            self.ui_state.clear_scene = false;
            self.scene.clear();
            self.sim.clear();
            self.ui_state.open_menu = None;
        }

        let show_place_devices_ui = self.chips_in_hand.len() > 0;

        let mut scene_hovered = !input.area_hovered(self.device_list_ui)
            && !input.area_hovered(self.overlay_ui)
            && self.ui_state.open_menu.is_none()
            && !(input.area_hovered(self.place_chips_ui) && show_place_devices_ui);

        self.sim.update();

        let mut model = Model::default();
        let mut painter = Painter::new(self.ui_state.create_style(), input, &mut model);
        painter.debug = self.ui_state.debug_ui;

        // ---- Draw Scene ----
        painter.set_transform(self.scene.transform);
        let scene_rs = self
            .scene
            .draw(&mut painter, &mut scene_hovered, &mut self.sim);
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
                self.sim.nodes[addr.0 as usize].set_source(src);
                self.start_wire = None;
            } else {
                log::info!("Toggling input");
                let state = self.sim.nodes[addr.0 as usize].state();
                self.sim.nodes[addr.0 as usize].set_state(!state);
            }
        }

        // ---- Draw Chip Placement Previews ----
        let mut place_chips = false;

        // Draw Confirm/Cancel UI
        if show_place_devices_ui {
            let pos = self.scene.transform * self.place_chips_pos;

            let mut cancel = false;
            show_device_preview_header(
                &mut self.place_chips_ui,
                &mut painter,
                pos,
                &mut place_chips,
                &mut cancel,
            );
            if cancel {
                self.chips_in_hand.clear();
            }
        }

        // Draw Chips
        let mut tmp_pos = self.place_chips_pos;
        for chip_idx in self.chips_in_hand.iter().copied() {
            let chip = &self.library.chips[chip_idx as usize];
            use scene::{DeviceImpl, Rotation};
            let mut preview = chip.preview(tmp_pos, Rotation::Rot0);

            preview.pos.y += preview.size().y * 0.5;
            let bounds = preview.bounds();
            painter.set_transform(self.scene.transform);
            preview.draw(None, &mut painter, &mut self.sim, &mut Default::default());
            tmp_pos.y += preview.size().y + 5.0;

            if painter.input.area_hovered(self.scene.transform * bounds) {
                scene_hovered = false;
            }

            painter.input.update_drag(
                Id::new("place_chips_ui"),
                self.scene.transform * bounds,
                self.place_chips_pos,
                PtrButton::LEFT,
            );
            if let Some(drag) = painter.input.get_drag_full(Id::new("place_chips_ui")) {
                let offset = drag.press_pos - self.scene.transform * drag.anchor;
                self.place_chips_pos =
                    self.scene.transform.inv() * (painter.input.ptr_pos() - offset);
            }

            if place_chips {
                place_chip(
                    &mut self.sim,
                    &mut self.scene,
                    None,
                    chip,
                    preview.pos,
                    scene::Rotation::Rot0,
                );
            }
        }
        if place_chips {
            self.chips_in_hand.clear();
        }

        painter.input.update_drag_hovered(
            Id::new("background"),
            scene_hovered,
            self.scene.transform.offset,
            PtrButton::LEFT,
        );
        if let Some(new_offset) = painter.input.get_drag(Id::new("background")) {
            self.scene.transform.offset = new_offset;
        }
        if let Some((anchor, delta)) = painter.input.zoom_delta() {
            self.scene.transform.zoom(anchor, delta * 0.1, 0.1..=100.0);
        }

        // ---- Draw UI & Menus ----
        match self.ui_state.open_menu {
            Some(Menu::Options) => {
                show_options_menu(
                    &mut self.options_menu,
                    &mut painter,
                    content_rect,
                    &mut self.ui_state,
                );
            }
            Some(Menu::Info) => {
                show_info_menu(
                    &mut self.info_menu,
                    &mut painter,
                    content_rect,
                    &mut self.ui_state,
                );
            }
            Some(Menu::Save) => {
                show_save_menu(
                    &mut self.save_menu,
                    &mut painter,
                    content_rect,
                    &mut self.ui_state,
                );
            }
            None => {
                show_overlay(
                    &mut self.overlay_ui,
                    &mut painter,
                    content_rect,
                    fps,
                    &mut self.ui_state,
                );

                let mut hold_chip = None;
                show_device_list(
                    &mut self.device_list_ui,
                    &mut painter,
                    content_rect,
                    &self.library.chips,
                    &mut hold_chip,
                );

                if let Some(idx) = hold_chip {
                    if self.chips_in_hand.is_empty() {
                        self.place_chips_pos = self.scene.transform.inv() * content_rect.center();
                    }
                    self.chips_in_hand.push(idx as u32);
                }
            }
        };

        // ---- Finish Drawing ----
        *text_input = painter.output.text_input.clone();
        let rs = renderer
            .render(
                gpu,
                Some(painter.style().background),
                [&model.upload(&gpu.device)],
            )
            .map_err(|_| format!("Failed to render models"));
        rs
    }
}

pub fn create_chip_save(name: String, sim: &sim::Sim, scene: &scene::Scene) -> ChipSave {
    let region_size = sim.next_region;
    let l_nodes = scene
        .left_nodes
        .states
        .iter()
        .map(|addr| (String::from(""), *addr, sim.get_node(*addr)))
        .collect();
    let r_nodes = scene
        .right_nodes
        .states
        .iter()
        .map(|addr| (String::from(""), *addr, sim.get_node(*addr)))
        .collect();
    let mut inner_nodes = Vec::new();
    for (_, device) in &scene.devices {
        for addr in device.sim_nodes() {
            inner_nodes.push((addr, sim.get_node(addr)));
        }
    }
    ChipSave {
        region_size,
        name,
        scene: None,
        l_nodes,
        r_nodes,
        inner_nodes,
    }
}

pub fn place_chip(
    sim: &mut sim::Sim,
    scene: &mut scene::Scene,
    save_id: Option<save::SaveId>,
    save: &save::ChipSave,
    pos: Vec2,
    orientation: scene::Rotation,
) {
    let mut l_nodes = vec![];
    let mut r_nodes = vec![];
    let mut inner_nodes = vec![];
    let region = sim.alloc_region(save.region_size);

    fn io_ty(node: &sim::Node) -> save::IoType {
        match node.source().ty() {
            sim::SourceTy::None => save::IoType::Input,
            _ => save::IoType::Output,
        }
    }

    for (name, addr, state) in &save.l_nodes {
        let addr = region.map(*addr);
        sim.set_node(addr, region.map_node(*state));
        l_nodes.push((addr, name.clone(), io_ty(state)));
    }
    for (name, addr, state) in &save.r_nodes {
        let addr = region.map(*addr);
        sim.set_node(addr, region.map_node(*state));
        r_nodes.push((addr, name.clone(), io_ty(state)));
    }
    for (addr, state) in &save.inner_nodes {
        let addr = region.map(*addr);
        sim.set_node(addr, region.map_node(*state));
        inner_nodes.push(addr);
    }

    let chip = scene::Chip {
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
