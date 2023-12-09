use crate::gpu::Gpu;
use glam::{ivec2, uvec2, IVec2, UVec2};

#[derive(Clone)]
pub struct Image {
    uv: (UVec2, UVec2),
    origin: IVec2,
}
impl Image {
    pub const ZERO: Self = Self {
        uv: (UVec2::ZERO, UVec2::ZERO),
        origin: IVec2::ZERO,
    };

    #[inline(always)]
    const fn new(x: u32, y: u32, w: u32, h: u32, ox: i32, oy: i32) -> Self {
        Self {
            uv: (uvec2(x, y), uvec2(x + w, y + h)),
            origin: ivec2(ox, oy),
        }
    }

    #[inline(always)]
    pub fn origin(&self) -> IVec2 {
        self.origin
    }

    #[inline(always)]
    pub fn size(&self) -> UVec2 {
        self.uv.1 - self.uv.0
    }
    #[inline(always)]
    pub fn uv_coords(&self) -> [UVec2; 4] {
        [
            uvec2(self.uv.0.x, self.uv.0.y),
            uvec2(self.uv.1.x, self.uv.0.y),
            uvec2(self.uv.1.x, self.uv.1.y),
            uvec2(self.uv.0.x, self.uv.1.y),
        ]
    }
}

pub struct FontKey<'a> {
    pub name: &'a str,
    pub size: u32,
    pub bold: bool,
    pub italic: bool,
}
impl<'a> FontKey<'a> {
    pub const fn new(name: &'a str, size: u32, bold: bool, italic: bool) -> Self {
        Self {
            name,
            size,
            bold,
            italic,
        }
    }
}

pub struct StaticFont(&'static [Image]);
impl StaticFont {
    pub fn get_char_image(&self, ch: char) -> &Image {
        self.0.get(ch as usize).unwrap_or(&self.0[0])
    }
}

pub struct StaticAtlasData {
    pub file: &'static [u8],
    pub size: u32,
    pub replacement_image: Image,
    pub white: Image,
    pub images: &'static [(&'static str, Image)],
    pub fonts: &'static [(FontKey<'static>, StaticFont)],
}
impl StaticAtlasData {
    pub fn get_image(&self, name: &str) -> &Image {
        self.images
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_key, img)| img)
            .unwrap_or(&self.replacement_image)
    }

    pub fn get_font(
        &self,
        _size: u32,
        bold: bool,
        italic: bool,
    ) -> &(FontKey<'static>, StaticFont) {
        self.fonts
            .iter()
            .find(|(key, _font)| key.bold == bold && key.italic == italic)
            .unwrap_or(&self.fonts[0])
    }
}
impl std::ops::Index<&str> for StaticAtlasData {
    type Output = Image;
    fn index(&self, name: &str) -> &Image {
        self.get_image(name)
    }
}

// Should define `static MAIN_ATLAS: StaticAtlasData`
include!("../../include/atlas_data.rs");

pub struct Atlas {
    pub handle: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}
impl Atlas {
    pub fn new(gpu: &Gpu) -> Self {
        use wgpu::*;
        let size = Extent3d {
            width: MAIN_ATLAS.size,
            height: MAIN_ATLAS.size,
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

        let atlas = image::load_from_memory(MAIN_ATLAS.file)
            .unwrap()
            .into_rgba8();

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
                bytes_per_row: Some(4 * MAIN_ATLAS.size),
                rows_per_image: Some(MAIN_ATLAS.size),
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
