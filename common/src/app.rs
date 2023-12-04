use crate::gpu::Gpu;
use crate::graphics::ui::{Align, Align2, CycleState, Painter, Placer, Style, TextField};
use crate::graphics::{Color, Font, Model, Rect, Renderer, TexCoords};
use crate::input::{InputState, PtrButton, TextInputState};
use crate::sim::{self, save, scene, Sim};
use crate::Id;

use glam::{UVec2, Vec2};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::time::SystemTime;
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
    frame_count: u32,
    ui_size: UiScale,
    ui_theme: UiTheme,
    chip_ty: ChipTy,
    chip_name: TextField,
    open_menu: Option<Menu>,
    fps: u32,
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            frame_count: 0,
            ui_size: UiScale::One,
            ui_theme: UiTheme::Dark,
            chip_ty: ChipTy::Sequential,
            chip_name: TextField::default(),
            open_menu: None,
            fps: 0,
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
        rs.seperator_w *= scale;

        if matches!(self.ui_theme, UiTheme::Light) {
            rs.text_color = Color::shade(0).into();
            rs.background = Color::shade(255 - 6 * 5);
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

#[derive(Default)]
pub struct OptionsMenu {
    background: Rect,
    title: Rect,
    title_sep: Rect,
    info: Rect,
    ui_size: Rect,
    ui_theme: Rect,
    frame_count: Rect,
    glyph_cache_count: Rect,
    done: Rect,
}
impl OptionsMenu {
    fn new(style: &Style, view_bounds: Rect) -> Self {
        let mut placer = Placer::new(
            &style,
            view_bounds.center(),
            Align2::CENTER,
            Align2::CENTER,
            Vec2::Y,
        );
        let mut rs: Self = Self::default();
        while placer.start() {
            rs.background = placer.bounds;
            rs.title = placer.next();
            rs.title_sep = placer.seperator();
            rs.info = placer.next();
            rs.ui_size = placer.next();
            rs.ui_theme = placer.next();
            rs.frame_count = placer.next();
            rs.glyph_cache_count = placer.next();
            rs.done = placer.next();
        }
        rs
    }
    fn show(&self, painter: &mut Painter, state: &mut UiState, rebuild: &mut bool) {
        painter.menu_background(self.background);
        painter.text(self.title, "Options");
        painter.seperator(self.title_sep);
        if painter.button(self.info, "Info").triggered {
            state.open_menu = Some(Menu::Info);
        }
        painter.cycle(self.ui_size, &mut state.ui_size, rebuild);
        painter.cycle(self.ui_theme, &mut state.ui_theme, &mut false);
        painter.text(self.frame_count, format!("frames: {}", state.frame_count));
        painter.text(
            self.glyph_cache_count,
            format!("cached glyphs: {}", painter.font.glyph_model_cache.len()),
        );
        if painter.button(self.done, "Done").triggered {
            state.open_menu = None;
        }
    }
}

#[derive(Default)]
pub struct OverlayUi {
    background: Rect,
    options: Rect,
    save: Rect,
    fps: Rect,
}
impl OverlayUi {
    fn new(style: &Style, view_bounds: Rect) -> Self {
        let mut placer = Placer::new(
            &style,
            view_bounds.min,
            Align2::TOP_LEFT,
            Align2::MIN,
            Vec2::X,
        );
        Self {
            options: placer.image_button(),
            save: placer.image_button(),
            fps: placer.next(),
            background: placer.bounds,
        }
    }

    fn show(&self, painter: &mut Painter, state: &mut UiState) {
        painter.menu_background(self.background);
        if painter
            .image_button(self.options, &TexCoords::OPTIONS)
            .triggered
        {
            state.open_menu = Some(Menu::Options);
        }
        if painter.image_button(self.save, &TexCoords::SAVE).triggered {
            state.open_menu = Some(Menu::Save);
        }
        painter.text(self.fps, format!("fps: {}", state.fps));
    }
}

#[derive(Default)]
pub struct SaveMenu {
    background: Rect,
    title: Rect,
    title_sep: Rect,
    name: Rect,
    chip_ty: Rect,
    done: Rect,
}
impl SaveMenu {
    fn new(style: &Style, view_bounds: Rect) -> Self {
        let mut placer = Placer::new(
            &style,
            view_bounds.center(),
            Align2::CENTER,
            Align2::CENTER,
            Vec2::Y,
        );
        let mut rs = Self::default();
        while placer.start() {
            rs.background = placer.bounds;
            rs.title = placer.next();
            rs.title_sep = placer.seperator();
            rs.name = placer.next();
            rs.chip_ty = placer.next();
            rs.done = placer.next();
        }
        rs
    }

    fn show(&self, painter: &mut Painter, state: &mut UiState) {
        painter.menu_background(self.background);
        painter.text(self.title, "Save to Chip");
        painter.seperator(self.title_sep);
        painter.text_edit(self.name, "Name", &mut state.chip_name);
        painter.cycle(self.chip_ty, &mut state.chip_ty, &mut false);
        if painter.button(self.done, "Done").triggered {
            state.open_menu = None;
        }
    }
}

#[derive(Default)]
pub struct InfoMenu {
    background: Rect,
    title: Rect,
    title_sep: Rect,
    lines: [Rect; 4],
    done: Rect,
}
impl InfoMenu {
    fn new(style: &Style, view_bounds: Rect) -> Self {
        let mut placer = Placer::new(
            &style,
            view_bounds.center(),
            Align2::CENTER,
            Align2::CENTER,
            Vec2::Y,
        );
        let mut rs = Self::default();
        while placer.start() {
            rs.background = placer.bounds;
            rs.title = placer.next();
            rs.title_sep = placer.seperator();
            rs.lines = std::array::from_fn(|_| placer.next());
            rs.done = placer.next();
        }
        rs
    }

    fn show(&self, painter: &mut Painter, state: &mut UiState) {
        painter.menu_background(self.background);
        painter.text(self.title, "Info");
        painter.seperator(self.title_sep);

        painter.text(self.lines[0], "Developer: Mason Feurer");
        painter.text(self.lines[1], "Version: 0.0.1");
        painter.text(
            self.lines[2],
            format!(
                "Platform: {} ({})",
                std::env::consts::OS,
                std::env::consts::FAMILY
            ),
        );
        painter.text(
            self.lines[3],
            format!("Architecture: {}", std::env::consts::ARCH),
        );

        if painter.button(self.done, "Done").triggered {
            state.open_menu = Some(Menu::Options);
        }
    }
}

#[derive(Default)]
pub struct PlaceChipsUi {
    background: Rect,
    confirm: Rect,
    cancel: Rect,
}
impl PlaceChipsUi {
    fn new(style: &Style, bottom_center: Vec2) -> Self {
        let mut style = style.clone();
        style.item_align = Align::Min;

        let mut placer = Placer::new(
            &style,
            bottom_center,
            Align2::BOTTOM_CENTER,
            Align2::MIN,
            Vec2::X,
        );
        let mut rs = Self::default();
        while placer.start() {
            rs.background = placer.bounds;
            rs.confirm = placer.image_button();
            rs.cancel = placer.image_button();
        }
        rs
    }

    fn show(&self, painter: &mut Painter, place_chips: &mut bool, cancel_place_chips: &mut bool) {
        painter.menu_background(self.background);
        if painter
            .image_button(self.confirm, &TexCoords::CONFIRM)
            .triggered
        {
            *place_chips = true;
        }
        if painter
            .image_button(self.cancel, &TexCoords::CANCEL)
            .triggered
        {
            *cancel_place_chips = true;
        }
    }
}

#[derive(Default)]
pub struct DeviceListUi {
    background: Rect,
    title: Rect,
    title_sep: Rect,
    chips: Vec<Rect>,
}
impl DeviceListUi {
    fn new(style: &Style, view_bounds: Rect, chips: &[save::ChipSave]) -> Self {
        let mut style = style.clone();
        style.item_align = Align::Max;
        style.item_size.x *= 0.8;

        let mut placer = Placer::new(
            &style,
            view_bounds.tr(),
            Align2::TOP_RIGHT,
            Align2::MIN,
            Vec2::Y,
        );
        let mut rs = Self::default();
        while placer.start() {
            rs.background = placer.bounds;
            rs.title = placer.next();
            rs.title_sep = placer.seperator();
            rs.chips.clear();
            for _ in chips {
                rs.chips.push(placer.next());
            }
        }
        rs
    }

    fn show(
        &self,
        painter: &mut Painter,
        chips: &[save::ChipSave],
        preview_chip: &mut Option<usize>,
    ) {
        painter.menu_background(self.background);
        painter.text(self.title, "Devices");
        painter.seperator(self.title_sep);
        for (idx, chip) in self.chips.iter().copied().enumerate() {
            if painter.button(chip, &chips[idx].name).triggered {
                *preview_chip = Some(idx);
            }
        }
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

    pub options_menu: OptionsMenu,
    pub save_menu: SaveMenu,
    pub info_menu: InfoMenu,
    pub overlay_ui: OverlayUi,
    pub device_list_ui: DeviceListUi,
    pub place_chips_ui: PlaceChipsUi,

    pub chip_saves: Vec<save::ChipSave>,
    pub chips_in_hand: Vec<u32>,
    pub ui_state: UiState,
    pub place_chips_pos: Vec2,
    pub font: Font<'static>,
    pub last_fps_update: Option<SystemTime>,
    pub frame_count: u32,
    pub painter_model: Model,
    pub layouts_dirty: bool,
}
impl App {
    pub fn new() -> Self {
        let mut app = Self::default();
        let nand_table = sim::TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            name: "Nand".into(),
            map: Box::new([1, 0, 0, 0]),
        };
        let not_table = sim::TruthTable {
            num_inputs: 1,
            num_outputs: 1,
            name: "Not".into(),
            map: Box::new([1, 0]),
        };
        app.sim.tables = vec![nand_table, not_table];
        let nand = save::ChipSave {
            region_size: 3,
            name: "Nand".into(),
            scene: None,
            l_nodes: vec![
                ("a".into(), sim::NodeAddr(0), sim::Source::new_none()),
                ("b".into(), sim::NodeAddr(1), sim::Source::new_none()),
            ],
            r_nodes: vec![(
                "out".into(),
                sim::NodeAddr(2),
                sim::Source::new_table(sim::TruthTableSource {
                    inputs: sim::NodeAddr(0),
                    output: 0,
                    id: sim::TruthTableId(0),
                }),
            )],
            inner_nodes: vec![],
        };
        let not = save::ChipSave {
            region_size: 2,
            name: "Not".into(),
            scene: None,
            l_nodes: vec![("in".into(), sim::NodeAddr(0), sim::Source::new_none())],
            r_nodes: vec![(
                "out".into(),
                sim::NodeAddr(1),
                sim::Source::new_table(sim::TruthTableSource {
                    inputs: sim::NodeAddr(0),
                    output: 0,
                    id: sim::TruthTableId(1),
                }),
            )],
            inner_nodes: vec![],
        };
        app.chip_saves.extend([nand, not]);
        app.last_fps_update = Some(SystemTime::now());
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
        self.layouts_dirty = true;
        self.place_chips_pos = win_size.as_vec2() * 0.5;

        self.gpu = Some(gpu);
        self.renderer = Some(renderer);
    }

    pub fn size(&self) -> UVec2 {
        self.gpu
            .as_ref()
            .map(|gpu| gpu.surface_size())
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
        self.layouts_dirty = true;
    }

    pub fn draw_frame(
        &mut self,
        input: &mut InputState,
        content_rect: Rect,
        text_input: &mut Option<TextInputState>,
    ) -> Result<(), String> {
        let gpu = self.gpu.as_ref().ok_or(format!("Missing Gpu instance"))?;
        let renderer = self
            .renderer
            .as_mut()
            .ok_or(format!("Missing Renderer instance"))?;

        // Update FPS counter:
        'f: {
            let Some(last_update) = &mut self.last_fps_update else {
                break 'f;
            };

            self.frame_count += 1;
            if SystemTime::now()
                .duration_since(*last_update)
                .unwrap()
                .as_secs()
                >= 1
            {
                *last_update = SystemTime::now();
                self.ui_state.fps = self.frame_count;
                self.frame_count = 0;
            }
        }

        let style = self.ui_state.create_style();
        if self.layouts_dirty {
            self.options_menu = OptionsMenu::new(&style, content_rect);
            self.save_menu = SaveMenu::new(&style, content_rect);
            self.info_menu = InfoMenu::new(&style, content_rect);
            self.overlay_ui = OverlayUi::new(&style, content_rect);
            self.device_list_ui = DeviceListUi::new(&style, content_rect, &self.chip_saves);
            log::info!("Generating UI layouts");
        }
        self.layouts_dirty = false;
        self.ui_state.frame_count += 1;

        let show_place_devices_ui = self.chips_in_hand.len() > 0;

        let mut scene_hovered = !input.area_hovered(self.device_list_ui.background)
            && !input.area_hovered(self.overlay_ui.background)
            && self.ui_state.open_menu.is_none()
            && !(input.area_hovered(self.place_chips_ui.background) && show_place_devices_ui);

        self.sim.update();

        // ----------------- Start Drawing --------------------
        if self.font.should_purge() {
            self.font.purge();
        }
        self.painter_model.clear();
        let mut painter = Painter::new(&style, input, &mut self.painter_model, &mut self.font);

        // ---- Draw Scene ----
        self.scene
            .draw(&mut painter, &mut scene_hovered, &mut self.sim);

        // ---- Draw Chip Placement Previews ----
        let mut place_chips = false;

        // Draw Confirm/Cancel UI
        if show_place_devices_ui {
            let pos = self.scene.transform.transform().apply(self.place_chips_pos);
            self.place_chips_ui = PlaceChipsUi::new(&style, pos);
            let mut cancel_place_chips = false;
            self.place_chips_ui
                .show(&mut painter, &mut place_chips, &mut cancel_place_chips);
            if cancel_place_chips {
                self.chips_in_hand.clear();
            }
        }

        // Draw Chips
        painter.set_transform(self.scene.transform.transform());
        let mut tmp_pos = self.place_chips_pos;
        for chip_idx in self.chips_in_hand.iter().copied() {
            let chip = &self.chip_saves[chip_idx as usize];
            use scene::{DeviceImpl, Rotation};
            let mut preview = chip.preview(tmp_pos, Rotation::Rot0);

            preview.pos.y += preview.size().y * 0.5;
            let bounds = preview.bounds();
            preview.draw(None, &mut painter, &mut self.sim);
            tmp_pos.y += preview.size().y + 5.0;

            if painter
                .input
                .area_hovered(self.scene.transform.transform().apply2(bounds))
            {
                scene_hovered = false;
            }

            painter.input.update_drag(
                Id::new("place_chips_ui"),
                self.scene.transform.transform().apply2(bounds),
                self.place_chips_pos,
                PtrButton::LEFT,
            );
            if let Some(drag) = painter.input.get_drag_full(Id::new("place_chips_ui")) {
                let offset = drag.press_pos - self.scene.transform.transform().apply(drag.anchor);
                self.place_chips_pos = self
                    .scene
                    .transform
                    .inv_transform()
                    .apply(painter.input.ptr_pos() - offset);
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
        painter.reset_transform();
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
        if painter.input.zoom_delta() != 0.0 {
            self.scene
                .transform
                .zoom(painter.input.ptr_pos(), painter.input.zoom_delta() * 0.2);
        }

        // ---- Draw UI & Menus ----
        match self.ui_state.open_menu {
            Some(Menu::Options) => {
                self.options_menu
                    .show(&mut painter, &mut self.ui_state, &mut self.layouts_dirty)
            }
            Some(Menu::Info) => self.info_menu.show(&mut painter, &mut self.ui_state),
            Some(Menu::Save) => self.save_menu.show(&mut painter, &mut self.ui_state),
            None => {
                self.overlay_ui.show(&mut painter, &mut self.ui_state);
                let mut hold_chip = None;
                self.device_list_ui
                    .show(&mut painter, &self.chip_saves, &mut hold_chip);
                if let Some(idx) = hold_chip {
                    if self.chips_in_hand.is_empty() {
                        self.place_chips_pos = self
                            .scene
                            .transform
                            .inv_transform()
                            .apply(content_rect.center());
                    }
                    self.chips_in_hand.push(idx as u32);
                }
            }
        };

        // ---- Finish Drawing ----
        let model = painter.upload(gpu);
        *text_input = painter.text_input.clone();

        let models = std::iter::once(&model);
        let rs = renderer
            .render(gpu, Some(style.background), models)
            .map_err(|_| format!("Failed to render models"));
        rs
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

    for (name, addr, source) in &save.l_nodes {
        let addr = region.map(*addr);
        sim.set_node_src(addr, region.map_src(*source));
        l_nodes.push((addr, name.clone()));
    }
    for (name, addr, source) in &save.r_nodes {
        let addr = region.map(*addr);
        sim.set_node_src(addr, region.map_src(*source));
        r_nodes.push((addr, name.clone()));
    }
    for (addr, source) in &save.inner_nodes {
        let addr = region.map(*addr);
        sim.set_node_src(addr, region.map_src(*source));
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
