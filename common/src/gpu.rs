use glam::{uvec2, UVec2};
use wgpu::*;

#[derive(Debug)]
pub enum GpuError {
    CreateSurfaceError(String),
    RequestAdapterError,
    RequestDeviceError(String),
}

pub struct Gpu {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
}
impl Gpu {
    pub async fn new(
        instance: &Instance,
        surface: Surface<'static>,
        size: UVec2,
    ) -> Result<Self, GpuError> {
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
            .expect("Surface should have config for this adapter");

        let limits = Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: limits,
                },
                None,
            )
            .await
            .map_err(|e| GpuError::RequestDeviceError(e.to_string()))?;
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
}
