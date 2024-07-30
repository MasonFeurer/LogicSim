use crate::gpu::Gpu;
use crate::settings::Settings;
use crate::sim::save::{ChipSave, IoType};
use crate::sim::scene::{Chip as SceneChip, Rotation, Scene};
use crate::sim::{Node, SourceTy};
use crate::ui::pages::{HomePage, Page, PageOutput};
use crate::Platform;

use egui::PlatformOutput;
use egui_wgpu::Renderer;
use glam::{UVec2, Vec2};

#[derive(Clone, Debug)]
pub struct AppInput {
    pub egui_input: egui::RawInput,
    pub fps: u32,
    pub win_size: UVec2,
    pub content_rect: egui::Rect,
}

pub struct App<P> {
    pub egui: egui::Context,
    pub gpu: Option<Gpu>,
    pub renderer: Option<Renderer>,
    pub prev_win_size: UVec2,
    pub settings: Settings,
    pub pages: Vec<Box<dyn Page<P>>>,
}
impl<P: Platform> Default for App<P> {
    fn default() -> Self {
        Self {
            egui: egui::Context::default(),
            gpu: None,
            renderer: None,
            prev_win_size: UVec2::ZERO,
            settings: Settings::default(),
            pages: vec![Box::new(HomePage)],
        }
    }
}
impl<P: Platform> App<P> {
    pub fn invalidate_surface(&mut self) {
        self.renderer = None;
        self.gpu = None;
    }

    pub async fn renew_surface(
        &mut self,
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        win_size: UVec2,
    ) {
        let gpu = Gpu::new(instance, surface, win_size).await.unwrap();
        gpu.configure_surface();

        let renderer = Renderer::new(&gpu.device, gpu.surface_config.format, None, 1);

        self.gpu = Some(gpu);
        self.renderer = Some(renderer);
        self.egui = egui::Context::default();
    }

    pub fn size(&self) -> UVec2 {
        self.gpu
            .as_ref()
            .map(Gpu::surface_size)
            .unwrap_or(UVec2::ZERO)
    }

    pub fn update_size(&mut self, size: UVec2) {
        self.prev_win_size = size;
        if let Some(gpu) = &mut self.gpu {
            gpu.surface_config.width = size.x;
            gpu.surface_config.height = size.y;
            gpu.configure_surface();
        }
    }

    pub fn draw_frame(&mut self, in_: AppInput) -> Result<PlatformOutput, String> {
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(String::from("Missing Gpu instance"))?;
        let renderer = self
            .renderer
            .as_mut()
            .ok_or(String::from("Missing Renderer instance"))?;

        if in_.win_size != self.prev_win_size {
            self.prev_win_size = in_.win_size;
            gpu.surface_config.width = in_.win_size.x;
            gpu.surface_config.height = in_.win_size.y;
            gpu.configure_surface();
        }

        // ---- Step Simulation ----
        // self.scenes[self.open_scene]
        //     .sim
        //     .update(&self.library.tables);

        let output = gpu.surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&Default::default());

        let egui_output = self.egui.run(in_.egui_input, |ctx| {
            // Update theme and scale
            ctx.set_visuals(crate::ui::create_visuals(self.settings.ui_theme));
            P::set_scale_factor(self.settings.ui_scale);

            let is_top_page = self.pages.len() == 1;
            let page = self.pages.last_mut().unwrap();
            let mut out = PageOutput::default();

            if !page.hide_top_panel() {
                egui::TopBottomPanel::top("top").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if page.has_back_button() && !is_top_page && ui.button("<").clicked() {
                            out.pop_page = true;
                        }
                        ui.label(page.title());
                    });
                });
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                page.draw(ui, &self.settings, &mut out);
            });
            if out.pop_page {
                let mut page = self.pages.pop().unwrap();
                page.on_close(&self.settings, &mut out);
            }
            if let Some(page) = out.push_page {
                self.pages.push(page);
            }
            if let Some(settings) = out.update_settings {
                self.settings = settings;
            }
        });

        for (id, delta) in egui_output.textures_delta.set {
            renderer.update_texture(&gpu.device, &gpu.queue, id, &delta);
        }
        for id in egui_output.textures_delta.free {
            renderer.free_texture(&id);
        }

        let clipped_prims = self
            .egui
            .tessellate(egui_output.shapes, egui_output.pixels_per_point);

        let mut encoder = gpu.device.create_command_encoder(&Default::default());

        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                in_.content_rect.width() as u32,
                in_.content_rect.height() as u32,
            ],
            pixels_per_point: egui_output.pixels_per_point,
        };

        _ = renderer.update_buffers(
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            &clipped_prims,
            &screen_desc,
        );

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("graphics-render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        renderer.render(&mut pass, &clipped_prims, &screen_desc);
        std::mem::drop(pass);

        gpu.queue.submit([encoder.finish()]);

        output.present();
        Ok(egui_output.platform_output)
    }
}

pub fn create_chip_save(scene: &Scene) -> ChipSave {
    let region_size = scene.sim.next_region;
    let l_nodes = scene
        .l_nodes
        .states
        .iter()
        .map(|(addr, name)| (name.clone(), *addr, scene.sim.get_node(*addr)))
        .collect();
    let r_nodes = scene
        .r_nodes
        .states
        .iter()
        .map(|(addr, name)| (name.clone(), *addr, scene.sim.get_node(*addr)))
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
