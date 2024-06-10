use super::{Atlas, Model, Vertex, VERTEX_ATTRIBUTES};
use crate::gpu::Gpu;

use wgpu::*;

static SHADER_SOURCE: &str = include_str!("../../include/shader.wgsl");

pub struct Uniform<T> {
    buffer: Buffer,
    _phantom: std::marker::PhantomData<T>,
}
impl<T> Uniform<T> {
    const T_SIZE: usize = std::mem::size_of::<T>();

    pub fn new(device: &Device, label: &str) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size: Self::T_SIZE as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn write(&self, queue: &Queue, t: &T) {
        let ptr = t as *const T as *const u8;
        let bytes = unsafe { std::slice::from_raw_parts(ptr, Self::T_SIZE) };

        queue.write_buffer(&self.buffer, 0, bytes);
    }
}

#[derive(Clone, Default)]
#[repr(C)]
pub struct Locals {
    pub screen_size: [f32; 2],
    pub texture_size: u32,
    _padding: [u32; 5],
}

pub struct Renderer {
    pub pipeline: RenderPipeline,
    pub bind_group: BindGroup,
    pub locals: Locals,

    pub atlas: Atlas,
    pub locals_buf: Uniform<Locals>,
}
impl Renderer {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let locals = Locals::default();
        let locals_buf = Uniform::new(device, "locals-buffer");
        let atlas = Atlas::new(gpu);

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("shader-module"),
            source: ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bind-group-layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("bind-group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: locals_buf.buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&atlas.view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&atlas.sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRIBUTES,
                }],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Cw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: gpu.surface_config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });
        Self {
            pipeline,
            bind_group,
            locals,
            atlas,
            locals_buf,
        }
    }

    pub fn upload_locals(&self, gpu: &Gpu) {
        self.locals_buf.write(&gpu.queue, &self.locals);
    }

    pub fn render<'a>(
        &mut self,
        gpu: &Gpu,
        clear: Option<super::Color>,
        models: impl IntoIterator<Item = &'a Model>,
    ) -> Result<(), SurfaceError> {
        let output = gpu.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());

        let mut cmd_encoder = gpu.device.create_command_encoder(&Default::default());

        let (load, store) = if let Some(color) = clear {
            let color = Color {
                r: color.r() as f64 / 255.0,
                g: color.g() as f64 / 255.0,
                b: color.b() as f64 / 255.0,
                a: color.a() as f64 / 255.0,
            };
            (LoadOp::Clear(color), StoreOp::Store)
        } else {
            (LoadOp::Load, StoreOp::Store)
        };

        let mut pass = cmd_encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("graphics-render-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations { load, store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);

        for model in models {
            pass.set_vertex_buffer(0, model.vertex_buf.slice(..));
            pass.set_index_buffer(model.index_buf.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..model.index_count, 0, 0..1);
        }

        std::mem::drop(pass);
        gpu.queue.submit([cmd_encoder.finish()]);

        output.present();
        Ok(())
    }
}
