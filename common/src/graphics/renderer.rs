use super::{Atlas, GpuModel, Transform, Vertex, VERTEX_ATTRIBUTES};

use crate::{gpu::Gpu, sim::Node, slice_as_byte_slice};
use glam::Vec2;
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

pub struct NodesBuffer {
    buffer: Buffer,
}
impl NodesBuffer {
    pub fn new(device: &Device, label: &str) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some(label),
            size: 1024 * std::mem::size_of::<Node>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    pub fn write(&self, queue: &Queue, nodes: &[Node]) {
        queue.write_buffer(&self.buffer, 0, unsafe { slice_as_byte_slice(nodes) });
    }
}

#[derive(Clone, Default)]
#[repr(C)]
pub struct Locals {
    pub node_state_color: [u32; 2],
    pub screen_size: [f32; 2],
    pub global_offset: [f32; 2],
    pub global_scale: f32,
    pub texture_size: u32,
}

pub struct Renderer {
    pub pipeline: RenderPipeline,
    pub bind_group: BindGroup,
    pub locals: Locals,

    pub atlas: Atlas,
    pub locals_buf: Uniform<Locals>,
    pub nodes_buf: NodesBuffer,
}
impl Renderer {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;

        fn buffer_binding(binding: u32, buffer: &Buffer) -> BindGroupEntry {
            BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            }
        }

        let locals = Locals::default();
        let locals_buf = Uniform::new(device, "locals-buffer");
        let nodes_buf = NodesBuffer::new(device, "nodes-buffer");
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
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
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
                buffer_binding(0, &locals_buf.buffer),
                buffer_binding(1, &nodes_buf.buffer),
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&atlas.view),
                },
                BindGroupEntry {
                    binding: 3,
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
                    format: TextureFormat::Rgba8UnormSrgb,
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
            nodes_buf,
            locals_buf,
        }
    }

    pub fn update_size(&mut self, gpu: &Gpu, size: Vec2) {
        self.locals.screen_size = size.into();
        self.locals_buf.write(&gpu.queue, &self.locals);
    }

    pub fn update_atlas_size(&mut self, gpu: &Gpu, size: u32) {
        self.locals.texture_size = size;
        self.locals_buf.write(&gpu.queue, &self.locals);
    }

    pub fn update_global_transform(&mut self, gpu: &Gpu, t: Transform) {
        self.locals.global_scale = t.scale;
        self.locals.global_offset = t.offset.into();
        self.locals_buf.write(&gpu.queue, &self.locals);
    }

    pub fn render<'a>(
        &mut self,
        gpu: &Gpu,
        clear: Option<super::Color>,
        models: impl IntoIterator<Item = &'a GpuModel>,
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
            (LoadOp::Clear(color), true)
        } else {
            (LoadOp::Load, true)
        };

        let mut pass = cmd_encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("graphics-render-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations { load, store },
            })],
            depth_stencil_attachment: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);

        for mesh in models {
            pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
            pass.set_index_buffer(mesh.index_buf.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count as u32, 0, 0..1);
        }

        std::mem::drop(pass);
        gpu.queue.submit([cmd_encoder.finish()]);

        output.present();
        Ok(())
    }
}
