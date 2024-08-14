use crate::save::{create_chip_from_scene, IoType, Project, StartingChip};
use crate::sim::scene::{BuiltinDeviceTy, NodeIdent, Scene, Wire, UNIT};
use crate::sim::{NodeAddr, Source};
use crate::{Platform, Settings};

use egui::Ui;
use glam::{vec2, Vec2};

pub struct PageOutput<P> {
    pub push_page: Option<Box<dyn Page<P>>>,
    pub pop_page: bool,
    pub update_settings: Option<Settings>,
    pub clicked_node: Option<(NodeIdent, NodeAddr, IoType)>,
    pub rclicked_node: Option<(NodeIdent, NodeAddr, IoType)>,
}
impl<P> Default for PageOutput<P> {
    fn default() -> Self {
        Self {
            push_page: None,
            pop_page: false,
            update_settings: None,
            clicked_node: None,
            rclicked_node: None,
        }
    }
}
impl<P> PageOutput<P> {
    pub fn push_page<Pa: Page<P> + 'static>(&mut self, page: Pa) {
        self.push_page = Some(Box::new(page));
    }

    pub fn replace_page<Pa: Page<P> + 'static>(&mut self, page: Pa) {
        self.pop_page = true;
        self.push_page = Some(Box::new(page));
    }
}

pub trait Page<P> {
    fn has_back_button(&self) -> bool {
        true
    }
    fn hide_top_panel(&self) -> bool {
        false
    }
    fn title(&self) -> String;
    fn draw(&mut self, ui: &mut Ui, settings: &Settings, out: &mut PageOutput<P>);
    fn on_close(&mut self, _settings: &Settings, _out: &mut PageOutput<P>) {}
}

pub struct HomePage;
impl<P: Platform> Page<P> for HomePage {
    fn title(&self) -> String {
        "Home".into()
    }

    fn draw(&mut self, ui: &mut Ui, settings: &Settings, out: &mut PageOutput<P>) {
        if ui.button("Select Project").clicked() {
            out.push_page(ProjectSelectPage::new::<P>())
        }
        if ui.button("Settings").clicked() {
            out.push_page(SettingsPage(settings.clone()));
        }
    }
}

pub struct ProjectSelectPage {
    projects: Vec<String>,
    load_err: Option<std::io::Error>,
    selected: Option<usize>,
    rename: Option<String>,
}
impl ProjectSelectPage {
    pub fn new<P: Platform>() -> Self {
        let (projects, load_err) = match P::list_available_projects() {
            Ok(projects) => (projects, None),
            Err(load_err) => (vec![], Some(load_err)),
        };
        Self {
            projects,
            load_err,
            selected: None,
            rename: None,
        }
    }

    pub fn reload<P: Platform>(&mut self) {
        let (projects, load_err) = match P::list_available_projects() {
            Ok(projects) => (projects, None),
            Err(load_err) => (vec![], Some(load_err)),
        };
        self.projects = projects;
        self.load_err = load_err;
        self.rename = None;
        self.selected = None;
    }
}
impl<P: Platform> Page<P> for ProjectSelectPage {
    fn title(&self) -> String {
        "Select a Project".into()
    }

    fn draw(&mut self, ui: &mut Ui, _settings: &Settings, out: &mut PageOutput<P>) {
        if let Some(err) = &self.load_err {
            ui.label(format!("Failed to load project(s) : {err:?}"));
        }
        ui.horizontal(|ui| {
            if ui.button("Create project").clicked() {
                out.push_page(ProjectCreatePage::default());
            }
            if P::can_open_dirs() && ui.button("Open Directory").clicked() {
                _ = P::open_save_dir();
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, project) in self.projects.iter().enumerate() {
                ui.horizontal(|ui| {
                    let selected = self.selected == Some(idx);
                    let mut rs = ui.button(project);
                    if selected {
                        rs = rs.highlight();
                    }
                    if rs.clicked() {
                        self.selected = Some(idx);
                    }
                });
            }
            let mut reload = false;
            ui.separator();
            if let Some(project) = self.selected.and_then(|i| self.projects.get(i)) {
                if let Some(new_name) = &mut self.rename {
                    ui.text_edit_singleline(new_name);
                } else {
                    ui.label(&*project);
                }
                ui.horizontal(|ui| {
                    if ui.button("open").clicked() {
                        match P::load_project(project) {
                            Err(err) => self.load_err = Some(err),
                            Ok(project) => {
                                out.pop_page = true;
                                out.push_page(WorkspacePage::new(project));
                            }
                        }
                    }
                    if let Some(new_name) = &self.rename {
                        if ui.button("save").clicked() {
                            P::rename_project(project, new_name);
                            reload = true;
                        }
                    } else {
                        if ui.button("rename").clicked() {
                            self.rename = Some(project.clone());
                        }
                    }
                    if ui.button("delete").clicked() {
                        P::delete_project(project);
                        reload = true;
                    }
                });
            }
            ui.add_space(50.0);
            if reload {
                self.reload::<P>();
            }
        });
    }
}

pub struct ProjectCreatePage {
    pub name: String,
    pub include_chips: [bool; StartingChip::COUNT as usize],
}
impl Default for ProjectCreatePage {
    fn default() -> Self {
        let mut include_chips = [false; StartingChip::COUNT as usize];
        include_chips[StartingChip::And as usize] = true;
        include_chips[StartingChip::Not as usize] = true;
        Self {
            name: String::from("Unnamed Project"),
            include_chips,
        }
    }
}
impl<P: Platform> Page<P> for ProjectCreatePage {
    fn title(&self) -> String {
        "Create a New Project".into()
    }

    fn draw(&mut self, ui: &mut Ui, _settings: &Settings, out: &mut PageOutput<P>) {
        ui.label("Name");
        ui.text_edit_singleline(&mut self.name);

        ui.label("Starting chips:");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for idx in 0..StartingChip::COUNT {
                    let name = format!("{:?}", StartingChip::from_u8(idx).unwrap());
                    ui.checkbox(&mut self.include_chips[idx as usize], name);
                }
            });
            ui.add_space(20.0);
        });

        ui.horizontal(|ui| {
            if ui.button("create").clicked() {
                let chips = self
                    .include_chips
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, b)| b.then_some(StartingChip::from_u8(idx as u8).unwrap()))
                    .collect::<Vec<_>>();
                let project = Project::new(self.name.clone(), chips);
                out.replace_page(WorkspacePage::new(project));
            }
            if ui.button("cancel").clicked() {
                out.pop_page = true;
            }
        });
    }
}

pub struct SettingsPage(Settings);
impl<P: Platform> Page<P> for SettingsPage {
    fn title(&self) -> String {
        "Settings".into()
    }

    fn draw(&mut self, ui: &mut Ui, _settings: &Settings, out: &mut PageOutput<P>) {
        use crate::settings::UiTheme;
        let set = &mut self.0;

        if ui.button("About").clicked() {
            out.push_page(InfoPage);
        }

        fn cycle<T: PartialEq + std::fmt::Debug + Clone>(
            ui: &mut Ui,
            label: &str,
            value: &mut T,
            options: &[T],
        ) {
            if options.into_iter().position(|v| v == &*value).is_none() {
                *value = options[0].clone();
            }
            let label = format!("{label}{value:?}");
            let rs = ui.button(label);
            if rs.clicked() {
                let idx = options.into_iter().position(|v| v == &*value).unwrap();
                let new_idx = (idx + 1) % options.len();
                *value = options[new_idx].clone();
            }
            if rs.secondary_clicked() {
                let idx = options.into_iter().position(|v| v == &*value).unwrap();
                if idx > 0 {
                    *value = options[idx - 1].clone();
                }
            }
        }
        cycle(
            ui,
            "Scale: ",
            &mut set.ui_scale,
            &[1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0],
        );
        cycle(
            ui,
            "Theme: ",
            &mut set.ui_theme,
            &[UiTheme::Light, UiTheme::Dark],
        );
        out.update_settings = Some(self.0.clone());
    }
}

pub struct InfoPage;
impl<P: Platform> Page<P> for InfoPage {
    fn title(&self) -> String {
        "About".into()
    }

    fn draw(&mut self, ui: &mut Ui, _settings: &Settings, _out: &mut PageOutput<P>) {
        ui.heading("Masons Logic Sim");
        ui.label("Version: indev (24-08-14)");
        ui.horizontal(|ui| {
            ui.label("Github: ");
            ui.hyperlink_to(
                "MasonFeurer/LogicSim",
                "https://github.com/MasonFeurer/LogicSim",
            );
        });
        let platform = format!(
            "Platform: {} ({})",
            std::env::consts::OS,
            std::env::consts::FAMILY
        );
        ui.label(platform);
        ui.label(format!("Architecture: {}", std::env::consts::ARCH));
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PlaceDevice {
    Builtin(BuiltinDeviceTy),
    Chip(usize),
}

#[derive(Clone, Copy, PartialEq)]
pub enum WorkspaceMenu {
    Options,
    CreateChip,
    Library,
}
impl WorkspaceMenu {
    pub fn show<P: Platform>(
        &self,
        page: &mut WorkspacePage,
        ui: &mut Ui,
        settings: &Settings,
        out: &mut PageOutput<P>,
    ) {
        fn button(ui: &mut Ui, label: impl Into<egui::WidgetText>) -> egui::Response {
            ui.add_sized([100.0, 20.0], egui::Button::new(label))
        }

        match self {
            Self::Options => {
                ui.heading("Options");
                ui.small("saved project");
                ui.separator();
                if button(ui, "Close").clicked() {
                    page.open_menu = None;
                }
                if button(ui, "Exit").clicked() {
                    out.pop_page = true;
                }
                let label = match page.snap_to_grid {
                    true => "Snap to grid: On",
                    false => "Snap to grid: Off",
                };
                if button(ui, label).clicked() {
                    page.snap_to_grid = !page.snap_to_grid;
                }

                let label = match page.show_grid {
                    true => "Show grid: On",
                    false => "Show grid: Off",
                };
                if button(ui, label).clicked() {
                    page.show_grid = !page.show_grid;
                }

                if button(ui, "Settings").clicked() {
                    out.push_page(SettingsPage(settings.clone()));
                }
            }
            Self::CreateChip => {
                ui.heading("Pack Into Chip");
                ui.separator();
                let scene = &mut page.project.scenes[page.open_scene as usize];
                ui.horizontal(|ui| {
                    ui.label("Name: ");
                    ui.text_edit_singleline(&mut scene.save_attrs.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Category: ");
                    ui.text_edit_singleline(&mut scene.save_attrs.category);
                });
                ui.horizontal(|ui| {
                    ui.label("Logic: ");
                    if button(ui, format!("{:?}", scene.save_attrs.logic)).clicked() {
                        scene.save_attrs.logic.cycle_in_place();
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        // self.scene.optimize();
                        let save =
                            create_chip_from_scene(&page.project.scenes[page.open_scene as usize]);
                        page.project.scenes.remove(page.open_scene as usize);
                        page.open_menu = None;

                        if let Some(c) = page
                            .project
                            .library
                            .chips
                            .iter_mut()
                            .find(|chip| chip.attrs.name == save.attrs.name)
                        {
                            *c = save;
                        } else {
                            page.project.library.chips.push(save);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        page.open_menu = None;
                    }
                });
            }
            Self::Library => _ = ui.heading("Library"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Corner {
    Tl,
    Bl,
    Br,
    Tr,
}

#[derive(Clone, Copy)]
pub struct DeviceCursor {
    pub pos: Vec2,
    /// which corner of the device will be placed at this location.
    pub corner: Corner,
}
impl Default for DeviceCursor {
    fn default() -> Self {
        Self {
            pos: vec2(0.0, 0.0),
            corner: Corner::Tl,
        }
    }
}

#[derive(Clone)]
pub struct WirePlacement {
    src: (NodeIdent, NodeAddr),
    anchors: Vec<Vec2>,
}

pub struct WorkspacePage {
    pub project: Project,
    pub snap_to_grid: bool,
    pub show_grid: bool,
    pub open_scene: usize,
    pub open_menu: Option<WorkspaceMenu>,
    pub items: Vec<(String, Vec<PlaceDevice>, bool)>,
    pub device_count: usize,

    pub cursor: DeviceCursor,
    pub wire_placement: Option<WirePlacement>,
}
impl WorkspacePage {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            show_grid: true,
            snap_to_grid: true,
            open_scene: 0,
            open_menu: None,
            items: vec![],
            device_count: 0,

            cursor: DeviceCursor::default(),
            wire_placement: None,
        }
    }
}
impl WorkspacePage {
    pub fn create_item_list(&mut self) {
        let mut cats = vec![(String::from("Builtin"), vec![], false)];
        let items = &mut cats[0].1;
        for idx in 0..BuiltinDeviceTy::COUNT {
            let device = BuiltinDeviceTy::from_u8(idx).unwrap();
            items.push(PlaceDevice::Builtin(device));
        }
        for category in self.project.library.categories() {
            cats.push((String::from(category), vec![], false));
            let items = &mut cats.last_mut().unwrap().1;
            for (lib_idx, _chip) in self.project.library.chips_in_category(category) {
                items.push(PlaceDevice::Chip(lib_idx));
            }
        }
        self.items = cats;
        self.device_count = self.project.library.chips.len();
    }

    pub fn toggle_menu(&mut self, menu: WorkspaceMenu) -> bool {
        if self.open_menu == Some(menu) {
            self.open_menu = None;
            false
        } else {
            self.open_menu = Some(menu);
            true
        }
    }

    fn place_device(&mut self, device: PlaceDevice) {
        let scene = &mut self.project.scenes[self.open_scene];
        let corner = self.cursor.corner;
        let center = self.cursor.pos;
        let size = match device {
            PlaceDevice::Builtin(builtin) => builtin.size(),
            PlaceDevice::Chip(id) => self.project.library.chips[id]
                .preview(center, Default::default())
                .size(),
        };
        self.cursor.pos.y += size.y;
        let center = match corner {
            Corner::Tl => vec2(center.x + size.x * 0.5, center.y + size.y * 0.5),
            Corner::Tr => vec2(center.x - size.x * 0.5, center.y + size.y * 0.5),
            Corner::Bl => vec2(center.x + size.x * 0.5, center.y - size.y * 0.5),
            Corner::Br => vec2(center.x - size.x * 0.5, center.y - size.y * 0.5),
        };

        log::info!("placing deivce: {device:?}");
        match device {
            PlaceDevice::Builtin(ty) => {
                use crate::sim::{scene, Node};

                let mut l_nodes = vec![];
                let mut r_nodes = vec![];
                let (input_count, output_count) = ty.io();
                let region = scene
                    .sim
                    .alloc_region(input_count as u32 + output_count as u32);

                for i in 0..input_count {
                    let addr = region.map(i as u32);
                    scene.sim.set_node(addr, Node::default());
                    l_nodes.push((addr, format!("in{i}"), IoType::Input));
                }
                for i in 0..output_count {
                    let addr = region.map(i as u32 + input_count as u32);
                    scene.sim.set_node(addr, Node::default());
                    r_nodes.push((addr, format!("out{i}"), IoType::Output));
                }

                let device = scene::BuiltinDevice {
                    ty,
                    region,
                    pos: center,
                    rotation: Default::default(),
                    l_nodes,
                    r_nodes,
                };
                scene.add_device(device);
            }
            PlaceDevice::Chip(id) => {
                use crate::sim::{scene, Node, SourceTy};

                let mut l_nodes = vec![];
                let mut r_nodes = vec![];
                let mut inner_nodes = vec![];
                let save = &self.project.library.chips[id];
                let region = scene.sim.alloc_region(save.region_size);

                fn io_ty(node: &Node) -> IoType {
                    match node.source().ty() {
                        SourceTy::NONE => IoType::Input,
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

                let chip = scene::Chip {
                    attrs: save.attrs.clone(),
                    region,
                    pos: center,
                    rotation: Default::default(),
                    save: Some(id),
                    l_nodes,
                    r_nodes,
                    inner_nodes,
                };
                scene.add_device(chip);
            }
        }
    }

    fn show_rpanel<P: Platform>(
        &mut self,
        ui: &mut Ui,
        _settings: &Settings,
        _out: &mut PageOutput<P>,
    ) {
        if self.project.library.chips.len() != self.device_count {
            self.create_item_list();
        }
        let mut place_device: Option<PlaceDevice> = None;

        let mut layout = ui.layout().clone();
        layout.cross_align = egui::Align::Center;
        ui.with_layout(layout, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (cat_name, _items, open) in &mut self.items {
                    let rs = ui.add_sized([80.0, 20.0], egui::Button::new(&*cat_name));
                    if rs.clicked() {
                        *open = !*open;
                    }
                    if *open {
                        rs.highlight();
                    }
                }

                for (_cat_name, items, open) in &self.items {
                    if !*open {
                        continue;
                    }
                    ui.separator();
                    for device in items {
                        let name = match device {
                            PlaceDevice::Chip(lib_idx) => {
                                self.project.library.chips[*lib_idx].attrs.name.clone()
                            }
                            PlaceDevice::Builtin(builtin) => format!("{builtin:?}"),
                        };
                        if ui
                            .add_sized([80.0, 20.0], egui::Button::new(&*name))
                            .clicked()
                        {
                            place_device = Some(*device);
                        }
                    }
                }
            });
        });
        if let Some(device) = place_device {
            self.place_device(device);
        }
    }

    fn show_tpanel<P: Platform>(
        &mut self,
        ui: &mut Ui,
        _settings: &Settings,
        _out: &mut PageOutput<P>,
    ) {
        if ui.button("options").clicked() {
            if self.toggle_menu(WorkspaceMenu::Options) {
                if let Err(err) = P::save_project(&self.project.name, self.project.clone()) {
                    log::warn!("Failed to save project {err:?}");
                }
            }
        }
        ui.label(&self.project.name);
        ui.separator();

        if ui.button("pack").clicked() {
            _ = self.toggle_menu(WorkspaceMenu::CreateChip);
        }
        ui.label("-");

        let mut rm_scene = None;
        for (scene_idx, scene) in self.project.scenes.iter().enumerate() {
            let rs = ui.add_enabled(
                self.open_scene != scene_idx,
                egui::Button::new(&scene.save_attrs.name),
            );
            if rs.clicked() {
                self.open_scene = scene_idx;
            }
            if rs.secondary_clicked() {
                rm_scene = Some(scene_idx);
            }
        }
        if let Some(scene_idx) = rm_scene {
            self.project.scenes.remove(scene_idx);
        }
        if ui.button("+").clicked() {
            self.open_scene = self.project.scenes.len();
            self.project.scenes.push(Scene::default());
        }
    }
}
impl<P: Platform> Page<P> for WorkspacePage {
    fn hide_top_panel(&self) -> bool {
        true
    }
    fn title(&self) -> String {
        "Workspace".into()
    }

    fn draw(&mut self, ui: &mut Ui, settings: &Settings, out: &mut PageOutput<P>) {
        if self.project.scenes.is_empty() {
            self.project.scenes = vec![Scene::default()];
            self.open_scene = 0;
        }

        // Show scene
        let scene_rs = if let Some(scene) = self.project.scenes.get_mut(self.open_scene) {
            let scene_rs = crate::ui::scene::show_scene(
                ui,
                &self.project.library,
                scene,
                self.snap_to_grid,
                self.show_grid,
                out,
            );

            // ----- Show Device Placing Cursor -----
            let t = scene.transform;
            let p = ui.painter();

            {
                let DeviceCursor { pos, corner } = self.cursor;
                let pos = egui::pos2(pos.x, pos.y);
                let rect = t * egui::Rect::from_min_size(pos, egui::vec2(UNIT, UNIT));

                let stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
                p.line_segment([rect.min, egui::pos2(rect.min.x, rect.max.y)], stroke);
                p.line_segment([rect.min, egui::pos2(rect.max.x, rect.min.y)], stroke);

                let rs = ui.interact(
                    rect,
                    egui::Id::from("cursor"),
                    egui::Sense::click_and_drag(),
                );
                self.cursor.pos.x += t.inv() * rs.drag_delta().x;
                self.cursor.pos.y += t.inv() * rs.drag_delta().y;

                if self.snap_to_grid && rs.drag_stopped() {
                    let u = UNIT;
                    self.cursor.pos = u * (self.cursor.pos / u).round();
                }
            }
            Some(scene_rs)
        } else {
            self.open_scene -= 1;
            None
        };

        // Update placing wire
        if let Some((ident, addr, _ty)) = out.rclicked_node {
            if let Some(WirePlacement { src, anchors }) = self.wire_placement.clone() {
                if src.0 != ident {
                    self.wire_placement = None;

                    let scene = &mut self.project.scenes[self.open_scene as usize];

                    _ = scene.rm_wire_by_target(addr);

                    let new_src = Source::new_addr(src.1);
                    scene.sim.nodes[addr.0 as usize].set_source(new_src);
                    scene.wires.push(Wire {
                        input: src.0,
                        output: ident,
                        anchors,
                    });
                }
            } else {
                self.wire_placement = Some(WirePlacement {
                    src: (ident, addr),
                    anchors: vec![],
                });
            }
        }
        if let Some((_ident, addr, ty)) = out.clicked_node {
            if matches!(ty, IoType::Input) {
                let scene = &mut self.project.scenes[self.open_scene as usize];
                let node = scene.sim.get_node(addr);
                scene.sim.set_node(addr, node.toggle_state());
            }
        }

        // ---- Place Wire Anchors
        if let Some(bg_rs) = scene_rs {
            if bg_rs.clicked() {
                let scene = &mut self.project.scenes[self.open_scene as usize];
                let ptr_pos = bg_rs.interact_pointer_pos().unwrap();
                let ptr_pos = vec2(ptr_pos.x, ptr_pos.y);
                if let Some(WirePlacement { anchors, .. }) = &mut self.wire_placement {
                    anchors.push(scene.transform.inv() * ptr_pos);
                }
            }
        }

        // ---- Draw Wire Being Placed ----
        if let Some(WirePlacement { src, anchors }) = &self.wire_placement {
            let scene = &mut self.project.scenes[self.open_scene as usize];
            if let Some(info) = scene.node_info(src.0) {
                let state = scene.sim.get_node(info.addr).state();

                let dst = ui.ctx().pointer_latest_pos().unwrap_or(egui::Pos2::ZERO);
                let dst = scene.transform.inv() * vec2(dst.x, dst.y);
                crate::ui::scene::draw_wire(
                    ui,
                    scene.transform,
                    state,
                    true,
                    info.pos,
                    dst,
                    anchors,
                );
            }
        }

        // Show top Panel
        ui.horizontal(|ui| {
            self.show_tpanel(ui, settings, out);
        });

        // Show right panel
        {
            let screen_rect = ui.ctx().screen_rect();
            let rpanel_w = 100.0;
            let rpanel_rect = egui::Rect::from_min_size(
                egui::pos2(screen_rect.width() - rpanel_w, 0.0),
                egui::vec2(rpanel_w, screen_rect.height()),
            );
            let mut rpanel_ui = ui.child_ui(rpanel_rect, ui.layout().clone(), None);

            egui::Frame::menu(ui.style()).show(&mut rpanel_ui, |ui| {
                self.show_rpanel(ui, settings, out);
            });
        }

        // Show menu if one is open
        if let Some(menu) = self.open_menu {
            egui::Window::new("menu")
                .anchor(egui::Align2::CENTER_CENTER, [0.0; 2])
                .resizable(false)
                .collapsible(false)
                .title_bar(false)
                .max_width(200.0)
                .show(ui.ctx(), |ui| {
                    let layout = ui.layout().clone().with_cross_align(egui::Align::Center);
                    ui.with_layout(layout, |ui| menu.show(self, ui, settings, out));
                });
        }
    }
}
