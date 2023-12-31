use super::{Color, Image, Rect, Transform, MAIN_ATLAS};
use glam::{vec2, UVec2, Vec2};

pub type Index = u32;

pub const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32x2, 2 => Uint32];

#[derive(Clone)]
#[repr(C)]
pub struct Vertex {
    pos: [f32; 2],
    uv: [u32; 2],
    color: u32,
}
impl Vertex {
    pub fn new(pos: Vec2, uv: UVec2, color: Color) -> Self {
        Self {
            pos: [pos.x, pos.y],
            uv: [uv.x, uv.y],
            color: color.0,
        }
    }
}

pub struct Model {
    pub bounds: Rect,
    pub vertex_buf: wgpu::Buffer,
    pub vertex_count: u32,
    pub index_buf: wgpu::Buffer,
    pub index_count: u32,
}
impl Model {
    pub fn new(
        device: &wgpu::Device,
        bounds: Rect,
        vertices: &[Vertex],
        indices: &[Index],
    ) -> Self {
        pub unsafe fn slice_as_byte_slice<T>(a: &[T]) -> &[u8] {
            std::slice::from_raw_parts(a.as_ptr() as *const u8, std::mem::size_of_val(a))
        }

        use wgpu::util::{BufferInitDescriptor, DeviceExt as _};
        let vertex_buf = BufferInitDescriptor {
            label: None,
            contents: unsafe { slice_as_byte_slice(vertices) },
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        };
        let index_buf = BufferInitDescriptor {
            label: None,
            contents: unsafe { slice_as_byte_slice(indices) },
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        };
        Self {
            bounds,
            vertex_buf: device.create_buffer_init(&vertex_buf),
            vertex_count: vertices.len() as u32,
            index_buf: device.create_buffer_init(&index_buf),
            index_count: indices.len() as u32,
        }
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Default, Clone)]
pub struct ModelBuilder {
    pub transform: Transform,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
    bounds: Rect,
}
impl ModelBuilder {
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.transform = Transform::default();
        self.bounds = Rect::default();
    }

    pub fn raw_tri(&mut self, mut vertices: [Vertex; 3]) {
        vertices[0].pos = (self.transform * Vec2::from(vertices[0].pos)).into();
        vertices[1].pos = (self.transform * Vec2::from(vertices[1].pos)).into();
        vertices[2].pos = (self.transform * Vec2::from(vertices[2].pos)).into();
        let i = self.vertices.len() as Index;
        for v in &vertices {
            self.bounds.expand_to_contain(v.pos.into());
        }
        self.vertices.extend(vertices);
        self.indices.extend(&[i, i + 1, i + 2]);
    }

    pub fn raw_quad(&mut self, mut vertices: [Vertex; 4]) {
        vertices[0].pos = (self.transform * Vec2::from(vertices[0].pos)).into();
        vertices[1].pos = (self.transform * Vec2::from(vertices[1].pos)).into();
        vertices[2].pos = (self.transform * Vec2::from(vertices[2].pos)).into();
        vertices[3].pos = (self.transform * Vec2::from(vertices[3].pos)).into();
        let i = self.vertices.len() as Index;
        for v in &vertices {
            self.bounds.expand_to_contain(v.pos.into());
        }
        self.vertices.extend(vertices);
        self.indices.extend(&[i, i + 1, i + 2, i, i + 2, i + 3]);
    }

    #[inline(always)]
    pub fn tri(&mut self, points: [Vec2; 3], tex: &Image, color: Color) {
        self.raw_tri([
            Vertex::new(points[0], tex.uv_coords()[0], color),
            Vertex::new(points[1], tex.uv_coords()[1], color),
            Vertex::new(points[2], tex.uv_coords()[2], color),
        ]);
    }

    #[inline(always)]
    pub fn quad(&mut self, points: [Vec2; 4], tex: &Image, color: Color) {
        self.raw_quad([
            Vertex::new(points[0], tex.uv_coords()[0], color),
            Vertex::new(points[1], tex.uv_coords()[1], color),
            Vertex::new(points[2], tex.uv_coords()[2], color),
            Vertex::new(points[3], tex.uv_coords()[3], color),
        ]);
    }

    #[inline(always)]
    pub fn line(&mut self, points: [Vec2; 2], w: f32, tex: &Image, color: Color) {
        let [a, b] = points;
        let p = (b - a).perp().normalize();
        let quad = [
            vec2(b.x - p.x * w * 0.5, b.y - p.y * w * 0.5),
            vec2(b.x + p.x * w * 0.5, b.y + p.y * w * 0.5),
            vec2(a.x + p.x * w * 0.5, a.y + p.y * w * 0.5),
            vec2(a.x - p.x * w * 0.5, a.y - p.y * w * 0.5),
        ];
        self.quad(quad, tex, color);
    }

    pub fn curve(&mut self, points: [Vec2; 3], detail: u32, w: f32, color: Color) {
        let [a, ctrl, b] = points;
        let mut prev_point = a;
        for step in 1..=detail {
            let t = step as f32 / detail as f32;
            let p = lerp_quad(a, ctrl, b, t);
            self.line([prev_point, p], w, &MAIN_ATLAS.white, color);
            prev_point = p;
        }
    }

    pub fn cubic_curve(&mut self, points: [Vec2; 4], detail: u32, w: f32, color: Color) {
        let [a, ctrl0, ctrl1, b] = points;
        let mut prev_point = a;
        for step in 1..=detail {
            let t = step as f32 / detail as f32;
            let p = lerp_cube(a, ctrl0, ctrl1, b, t);
            self.line([prev_point, p], w, &MAIN_ATLAS.white, color);
            prev_point = p;
        }
    }

    pub fn circle(&mut self, center: Vec2, r: f32, detail: u32, color: Color) {
        let tex = &MAIN_ATLAS.white;
        let mut prev_pos = center + vec2(0.0f32.sin(), 0.0f32.cos()) * r;
        for step in 1..=detail {
            let angle = (step as f32 / detail as f32) * std::f32::consts::TAU;
            let p = center + vec2(angle.sin(), angle.cos()) * r;
            self.tri([prev_pos, p, center], tex, color);
            prev_pos = p;
        }
    }

    pub fn circle_outline(&mut self, center: Vec2, r: f32, w: f32, detail: u32, color: Color) {
        let tex = &MAIN_ATLAS.white;
        let mut prev_pos = center + vec2(0.0f32.sin(), 0.0f32.cos()) * r;
        for step in 1..=detail {
            let angle = (step as f32 / detail as f32) * std::f32::consts::TAU;
            let p = center + vec2(angle.sin(), angle.cos()) * r;
            self.line([prev_pos, p], w, tex, color);
            prev_pos = p;
        }
    }

    pub fn circle_section(
        &mut self,
        center: Vec2,
        r: f32,
        detail: u32,
        range: [f32; 2],
        color: Color,
    ) {
        const TAU: f32 = std::f32::consts::TAU;
        let tex = &MAIN_ATLAS.white;
        let range_size = range[1] - range[0];
        let mut prev_pos = center + vec2((range[0] * TAU).sin(), (range[0] * TAU).cos()) * r;
        for step in 1..=detail {
            let angle = (range[0] + range_size * (step as f32 / detail as f32)) * TAU;
            let p = center + vec2(angle.sin(), angle.cos()) * r;
            self.tri([prev_pos, p, center], tex, color);
            prev_pos = p;
        }
    }

    pub fn circle_outline_section(
        &mut self,
        center: Vec2,
        r: f32,
        w: f32,
        detail: u32,
        range: [f32; 2],
        color: Color,
    ) {
        const TAU: f32 = std::f32::consts::TAU;
        let tex = &MAIN_ATLAS.white;
        let range_size = range[1] - range[0];
        let mut prev_pos = center + vec2((range[0] * TAU).sin(), (range[0] * TAU).cos()) * r;
        for step in 1..=detail {
            let angle = (range[0] + range_size * (step as f32 / detail as f32)) * TAU;
            let p = center + vec2(angle.sin(), angle.cos()) * r;
            self.line([prev_pos, p], w, tex, color);
            prev_pos = p;
        }
    }

    #[inline(always)]
    pub fn rect(&mut self, rect: Rect, tex: &Image, color: Color) {
        self.quad(rect.corners(), tex, color);
    }

    pub fn rect_outline(&mut self, rect: Rect, w: f32, color: Color) {
        let tex = &MAIN_ATLAS.white;
        self.line([rect.tl(), rect.tr()], w, tex, color);
        self.line([rect.tr(), rect.br()], w, tex, color);
        self.line([rect.bl(), rect.br()], w, tex, color);
        self.line([rect.tl(), rect.bl()], w, tex, color);
    }

    pub fn rounded_rect(&mut self, rect: Rect, r: f32, detail: u32, tex: &Image, color: Color) {
        let (tl, tr, bl, br) = (rect.tl(), rect.tr(), rect.bl(), rect.br());

        let rect = Rect::from_min_max(rect.min + r, rect.max - r);
        self.rect(rect, tex, color);

        let lrect = Rect::from_min_max(rect.tl() - Vec2::X * r, rect.bl());
        let trect = Rect::from_min_max(rect.tl() - Vec2::Y * r, rect.tr());
        let rrect = Rect::from_min_max(rect.br(), rect.tr() + Vec2::X * r);
        let brect = Rect::from_min_max(rect.bl() + Vec2::Y * r, rect.br());
        self.rect(lrect, tex, color);
        self.rect(trect, tex, color);
        self.rect(rrect, tex, color);
        self.rect(brect, tex, color);

        self.circle_section(tl + vec2(r, r), r, detail, [0.50, 0.75], color);
        self.circle_section(tr + vec2(-r, r), r, detail, [0.25, 0.50], color);
        self.circle_section(br - vec2(r, r), r, detail, [0.0, 0.25], color);
        self.circle_section(bl + vec2(r, -r), r, detail, [0.75, 1.0], color);
    }

    pub fn rounded_rect_outline(&mut self, rect: Rect, w: f32, r: f32, detail: u32, color: Color) {
        let tex = &MAIN_ATLAS.white;
        let (tl, tr, bl, br) = (rect.tl(), rect.tr(), rect.bl(), rect.br());

        self.circle_outline_section(tl + vec2(r, r), r, w, detail, [0.50, 0.75], color);
        self.circle_outline_section(tr + vec2(-r, r), r, w, detail, [0.25, 0.50], color);
        self.circle_outline_section(br - vec2(r, r), r, w, detail, [0.0, 0.25], color);
        self.circle_outline_section(bl + vec2(r, -r), r, w, detail, [0.75, 1.0], color);
        self.line([tl + Vec2::X * r, tr - Vec2::X * r], w, tex, color);
        self.line([tr + Vec2::Y * r, br - Vec2::Y * r], w, tex, color);
        self.line([bl + Vec2::X * r, br - Vec2::X * r], w, tex, color);
        self.line([tl + Vec2::Y * r, bl - Vec2::Y * r], w, tex, color);
    }

    pub fn finish(&self, device: &wgpu::Device) -> Model {
        Model::new(device, self.bounds, &self.vertices, &self.indices)
    }
}

#[inline(always)]
fn lerp_line(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    vec2(a.x - (a.x - b.x) * t, a.y - (a.y - b.y) * t)
}
#[inline(always)]
fn lerp_quad(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let a = lerp_line(p0, p1, t);
    let b = lerp_line(p1, p2, t);
    lerp_line(a, b, t)
}
#[inline(always)]
fn lerp_cube(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let a = lerp_quad(p0, p1, p2, t);
    let b = lerp_quad(p1, p2, p3, t);
    lerp_line(a, b, t)
}
