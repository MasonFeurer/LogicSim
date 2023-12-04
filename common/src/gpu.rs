use glam::{uvec2, UVec2};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use wgpu::*;

#[derive(Debug)]
pub enum GpuError {
    CreateSurfaceError(CreateSurfaceError),
    RequestAdapterError,
    RequestDeviceError,
}

pub struct Gpu {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub surface_config: SurfaceConfiguration,
}
impl Gpu {
    pub async fn new<W: HasRawWindowHandle + HasRawDisplayHandle>(
        instance: &Instance,
        window: &W,
        size: UVec2,
    ) -> Result<Self, GpuError> {
        let surface = unsafe { instance.create_surface(window) }
            .map_err(|e| GpuError::CreateSurfaceError(e))?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(GpuError::RequestAdapterError)?;

        let surface_config = surface
            .get_default_config(&adapter, size.x, size.y)
            .expect("Surface should have connfig for this adapter");

        let mut limits = Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        limits.max_storage_buffers_per_shader_stage = 1;
        limits.max_storage_buffer_binding_size = 128 << 20;

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits,
                },
                None,
            )
            .await
            .map_err(|_| GpuError::RequestDeviceError)?;
        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
        })
    }

    pub fn surface_size(&self) -> UVec2 {
        uvec2(self.surface_config.width, self.surface_config.height)
    }

    pub fn configure_surface(&self) {
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn create_buffer(&self, size: u64, usage: BufferUsages) -> Buffer {
        self.device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        })
    }
}
