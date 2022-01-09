#![feature(bigint_helper_methods)]

use std::fmt::Debug;
use std::iter;
use std::mem::size_of;
use std::num::NonZeroU64;

use bytemuck::Pod;
use bytemuck::Zeroable;
use num::Complex;
use wgpu::include_wgsl;
use wgpu::util::BufferInitDescriptor;
use wgpu::util::DeviceExt;
use wgpu::BindGroupDescriptor;
use wgpu::BindGroupEntry;
use wgpu::BindGroupLayoutDescriptor;
use wgpu::BindGroupLayoutEntry;
use wgpu::BindingResource;
use wgpu::BindingType;
use wgpu::Buffer;
use wgpu::BufferBinding;
use wgpu::BufferBindingType;
use wgpu::BufferUsages;
use wgpu::Color;
use wgpu::CommandEncoderDescriptor;
use wgpu::Device;
use wgpu::DeviceDescriptor;
use wgpu::FragmentState;
use wgpu::LoadOp;
use wgpu::Operations;
use wgpu::PipelineLayoutDescriptor;
use wgpu::PresentMode;
use wgpu::PrimitiveState;
use wgpu::PrimitiveTopology;
use wgpu::Queue;
use wgpu::RenderBundle;
use wgpu::RenderBundleDescriptor;
use wgpu::RenderBundleEncoderDescriptor;
use wgpu::RenderPassColorAttachment;
use wgpu::RenderPassDescriptor;
use wgpu::RenderPipelineDescriptor;
use wgpu::RequestAdapterOptions;
use wgpu::ShaderStages;
use wgpu::Surface;
use wgpu::SurfaceConfiguration;
use wgpu::TextureFormat;
use wgpu::TextureUsages;
use wgpu::TextureViewDescriptor;
use wgpu::VertexState;
use winit::window::Window;

pub mod num;

const ITERATIONS: u32 = 1200;
// The mandelbrot set ranges from -2 to 2, so multiplying that by 150 makes it take up a 600x600 space initially.
pub const INITIAL_ZOOM: f32 = 150.0;

#[derive(Clone, Copy, Zeroable, Pod, Debug)]
#[repr(C)]
pub struct Settings {
    center: [f32; 2],

    iterations: u32,

    camera: [f32; 2],
    zoom: f32,
}

#[derive(Debug)]
pub struct State {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,

    pub settings_buffer: Buffer,
    pub render_bundle: RenderBundle,
    pub swapchain_format: TextureFormat,

    // It's easier to keep a copy of these externally than read them from GPU memory every time.
    pub camera: Complex,
    pub zoom: f32,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                // Make sure this adapter can render to the window.
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                    limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .expect("Failed to obtain device");

        let size = window.inner_size();

        let settings_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Settings buffer"),
            contents: bytemuck::bytes_of(&Settings {
                center: [size.width as f32 / 2.0, size.height as f32 / 2.0],

                camera: [0.0, 0.0],
                zoom: INITIAL_ZOOM,

                iterations: ITERATIONS,
            }),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let settings_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Settings bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<Settings>() as u64),
                    },
                    count: None,
                }],
            });

        let settings_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Settings bind group"),
            layout: &settings_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &settings_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render pipeline layout"),
            bind_group_layouts: &[&settings_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(&include_wgsl!("shader.wgsl"));
        let swapchain_format = surface.get_preferred_format(&adapter).unwrap();

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[swapchain_format.into()],
            }),
        });

        let mut render_bundle_encoder =
            device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
                label: Some("Render bundle encoder"),
                color_formats: &[swapchain_format],
                depth_stencil: None,
                sample_count: 1,
            });

        render_bundle_encoder.set_pipeline(&render_pipeline);
        render_bundle_encoder.set_bind_group(0, &settings_bind_group, &[]);
        render_bundle_encoder.draw(0..4, 0..1);

        let render_bundle = render_bundle_encoder.finish(&RenderBundleDescriptor {
            label: Some("Render bundle"),
        });

        surface.configure(
            &device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: swapchain_format,
                width: size.width,
                height: size.height,
                present_mode: PresentMode::Mailbox,
            },
        );

        Self {
            device,
            queue,
            surface,

            settings_buffer,
            render_bundle,
            swapchain_format,

            camera: Complex::default(),
            zoom: INITIAL_ZOOM,
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        // Reconfigure the surface for the new size
        self.surface.configure(
            &self.device,
            &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: self.swapchain_format,
                width,
                height,
                present_mode: PresentMode::Mailbox,
            },
        );

        // Tell the GPU where the center of the screen now is
        self.queue.write_buffer(
            &self.settings_buffer,
            0,
            bytemuck::cast_slice(&[width as f32 / 2.0, height as f32 / 2.0]),
        );
    }

    pub fn render(&self) {
        let frame = self
            .surface
            .get_current_frame()
            .expect("Failed to acquire next swap chain texture")
            .output;

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Command encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        // TODO: do we need to clear the screen here?
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            rpass.execute_bundles(iter::once(&self.render_bundle));
        }

        self.queue.submit(Some(encoder.finish()));
    }

    /// Update the camera's position and zoom in the GPU's buffer to the latest values.
    pub fn update_camera(&self) {
        self.queue.write_buffer(
            &self.settings_buffer,
            8,
            bytemuck::cast_slice(&[self.camera.real, self.camera.imag, self.zoom]),
        )
    }

    // Gets the target length of components' subints given the current level of zoom.
    pub fn comp_size(&self) {
        
    }
}
