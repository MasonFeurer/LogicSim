use crate::{
    save::{Project, StartingChip},
    sim::scene::{BuiltinDeviceTy, Scene, UNIT},
};
use crate::{Platform, Settings};
use egui::Ui;
use glam::{vec2, Vec2};

pub struct PageOutput<P> {
    pub push_page: Option<Box<dyn Page<P>>>,
    pub pop_page: bool,
    pub update_settings: Option<Settings>,
}
impl<P> Default for PageOutput<P> {
    fn default() -> Self {
        Self {
            push_page: None,
            pop_page: false,
            update_settings: None,
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
        if ui.button("Open project").clicked() {
            match P::list_available_projects() {
                Ok(projects) => out.push_page(ProjectSelectPage {
                    projects,
                    load_err: None,
                }),
                Err(load_err) => out.push_page(ProjectSelectPage {
                    projects: vec![],
                    load_err: Some(load_err),
                }),
            }
        }
        if ui.button("Create project").clicked() {
            out.push_page(ProjectCreatePage::default());
        }
        if ui.button("Settings").clicked() {
            out.push_page(SettingsPage(settings.clone()));
        }
    }
}

pub struct ProjectSelectPage {
    projects: Vec<String>,
    load_err: Option<std::io::Error>,
}
impl<P: Platform> Page<P> for ProjectSelectPage {
    fn title(&self) -> String {
        "Select a Project".into()
    }

    fn draw(&mut self, ui: &mut Ui, _settings: &Settings, out: &mut PageOutput<P>) {
        if let Some(err) = &self.load_err {
            ui.label(format!("Failed to load project(s) : {err:?}"));
        }
        for project in &self.projects {
            if ui.button(project).clicked() {
                match P::load_project(project) {
                    Err(err) => self.load_err = Some(err),
                    Ok(project) => {
                        out.push_page(WorkspacePage::new(project));
                    }
                }
            }
        }
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
        ui.horizontal(|ui| {
            for idx in 0..StartingChip::COUNT {
                let name = format!("{:?}", StartingChip::from_u8(idx).unwrap());
                ui.checkbox(&mut self.include_chips[idx as usize], name);
            }
        });

        ui.add_space(20.0);

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
        if P::can_open_dirs() && ui.button("Open Save Directory").clicked() {
            _ = P::open_save_dir();
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
        }
        cycle(
            ui,
            "Scale: ",
            &mut set.ui_scale,
            &[0.25, 0.5, 0.75, 1.0, 1.25, 1.50, 1.75, 2.0],
        );
        cycle(
            ui,
            "Theme: ",
            &mut set.ui_theme,
            &[UiTheme::Light, UiTheme::Dark, UiTheme::Night],
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
        ui.heading("Logisim");
        ui.label("Version: indev (24-07-16)");
        ui.horizontal(|ui| {
            ui.label("Github: ");
            ui.hyperlink_to(
                "MasonFeurer/Logisim",
                "https://github.com/MasonFeurer/Logisim",
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
            Self::CreateChip => _ = ui.heading("Create Chip"),
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

pub struct WorkspacePage {
    pub project: Project,
    pub snap_to_grid: bool,
    pub show_grid: bool,
    pub open_scene: usize,
    pub open_menu: Option<WorkspaceMenu>,
    pub items: Vec<(String, Vec<PlaceDevice>, bool)>,
    pub cursor: DeviceCursor,
}
impl WorkspacePage {
    pub fn new(project: Project) -> Self {
        let mut cats = vec![(String::from("Builtin"), vec![], false)];
        let items = &mut cats[0].1;
        for idx in 0..BuiltinDeviceTy::COUNT {
            let device = BuiltinDeviceTy::from_u8(idx).unwrap();
            items.push(PlaceDevice::Builtin(device));
        }
        for category in project.library.categories() {
            cats.push((String::from(category), vec![], false));
            let items = &mut cats.last_mut().unwrap().1;
            for (lib_idx, _chip) in project.library.chips_in_category(category) {
                items.push(PlaceDevice::Chip(lib_idx));
            }
        }

        Self {
            project,
            show_grid: true,
            snap_to_grid: false,
            open_scene: 0,
            open_menu: None,
            cursor: DeviceCursor::default(),
            items: cats,
        }
    }
}
impl WorkspacePage {
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
        let center = match corner {
            Corner::Tl => vec2(center.x + size.x * 0.5, center.y + size.y * 0.5),
            Corner::Tr => vec2(center.x - size.x * 0.5, center.y + size.y * 0.5),
            Corner::Bl => vec2(center.x + size.x * 0.5, center.y - size.y * 0.5),
            Corner::Br => vec2(center.x - size.x * 0.5, center.y - size.y * 0.5),
        };

        log::info!("placing deivce: {device:?}");
        match device {
            PlaceDevice::Builtin(ty) => {
                use crate::save::IoType;
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
                use crate::save::IoType;
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
        let mut place_device: Option<PlaceDevice> = None;

        let mut layout = ui.layout().clone();
        layout.cross_align = egui::Align::Center;
        ui.with_layout(layout, |ui| {
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
        if let Some(scene) = self.project.scenes.get_mut(self.open_scene) {
            crate::ui::scene::show_scene(
                ui,
                &self.project.library,
                scene,
                self.snap_to_grid,
                self.show_grid,
            );

            // ----- Show Device Placing Cursor -----
            let t = scene.transform;
            let p = ui.painter();

            {
                let DeviceCursor { pos, corner: _ } = self.cursor;
                let pos = egui::pos2(pos.x, pos.y);
                let rect = t * egui::Rect::from_min_size(pos, egui::vec2(20.0, 20.0));

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
        } else {
            self.open_scene -= 1;
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
                .max_width(100.0)
                .show(ui.ctx(), |ui| {
                    let layout = ui.layout().clone().with_cross_align(egui::Align::Center);
                    ui.with_layout(layout, |ui| menu.show(self, ui, settings, out));
                });
        }
    }
}
