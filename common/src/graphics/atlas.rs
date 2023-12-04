use crate::gpu::Gpu;
use glam::{vec2, Vec2};

static ATLAS_FILE: &[u8] = include_bytes!("../../include/atlas2.png");
const ATLAS_SIZE: u32 = 128;

const N0: f32 = 0.0;
const N1: f32 = 1.0 / ATLAS_SIZE as f32;
const N39: f32 = 39.0 / ATLAS_SIZE as f32; // 38 + 1
const N77: f32 = 77.0 / ATLAS_SIZE as f32; // 38 * 2 + 1
const N115: f32 = 115.0 / ATLAS_SIZE as f32; // 38 * 3 + 1

pub const WHITE_TEX_COORDS: TexCoords = TexCoords {
    uv_coords: [vec2(N0, N0), vec2(N1, N0), vec2(N1, N1), vec2(N0, N1)],
};
pub const DOWN_TEX_COORDS: TexCoords = TexCoords {
    uv_coords: [vec2(N1, N1), vec2(N39, N1), vec2(N39, N39), vec2(N1, N39)],
};
pub const OPTIONS_TEX_COORDS: TexCoords = TexCoords {
    uv_coords: [vec2(N39, N1), vec2(N77, N1), vec2(N77, N39), vec2(N39, N39)],
};
pub const SAVE_TEX_COORDS: TexCoords = TexCoords {
    uv_coords: [
        vec2(N77, N1),
        vec2(N115, N1),
        vec2(N115, N39),
        vec2(N77, N39),
    ],
};
pub const CONFIRM_TEX_COORDS: TexCoords = TexCoords {
    uv_coords: [vec2(N1, N39), vec2(N39, N39), vec2(N39, N77), vec2(N1, N77)],
};
pub const CANCEL_TEX_COORDS: TexCoords = TexCoords {
    uv_coords: [
        vec2(N39, N39),
        vec2(N77, N39),
        vec2(N77, N77),
        vec2(N39, N77),
    ],
};

pub struct Atlas {
    pub handle: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}
impl Atlas {
    pub fn new(gpu: &Gpu) -> Self {
        use wgpu::*;
        let size = Extent3d {
            width: ATLAS_SIZE,
            height: ATLAS_SIZE,
            depth_or_array_layers: 1,
        };
        let handle = gpu.device.create_texture(&TextureDescriptor {
            label: Some("texture-atlas-handle"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = handle.create_view(&TextureViewDescriptor::default());
        let sampler = gpu.device.create_sampler(&SamplerDescriptor::default());

        let atlas = image::load_from_memory(ATLAS_FILE).unwrap().into_rgba8();

        gpu.queue.write_texture(
            ImageCopyTexture {
                texture: &handle,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            atlas.as_raw(),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * ATLAS_SIZE),
                rows_per_image: Some(ATLAS_SIZE),
            },
            size,
        );
        Self {
            handle,
            view,
            sampler,
        }
    }
}

#[derive(Clone)]
pub struct TexCoords {
    pub uv_coords: [Vec2; 4],
}
impl TexCoords {
    pub const WHITE: Self = WHITE_TEX_COORDS;
    pub const DOWN: Self = DOWN_TEX_COORDS;
    pub const OPTIONS: Self = OPTIONS_TEX_COORDS;
    pub const SAVE: Self = SAVE_TEX_COORDS;
    pub const CONFIRM: Self = CONFIRM_TEX_COORDS;
    pub const CANCEL: Self = CANCEL_TEX_COORDS;
}
